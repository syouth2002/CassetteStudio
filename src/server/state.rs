//! Library ownership, persistence, migration and background-job coordination.
use crate::{audio, model::*};
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Clone)]
pub(super) struct App {
    pub(super) _lock: Arc<fs::File>,
    pub(super) root: PathBuf,
    pub(super) library: Arc<Mutex<Library>>,
    pub(super) jobs: Arc<tokio::sync::Semaphore>,
}
pub(super) struct Error(pub(super) String);
impl From<String> for Error {
    fn from(v: String) -> Self {
        Self(v)
    }
}
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error":self.0})),
        )
            .into_response()
    }
}
pub(super) type Api<T> = Result<Json<T>, Error>;
impl App {
    pub(super) async fn ensure_loudness(&self, ids: &[String]) -> Result<Vec<Track>, Error> {
        if ids.len() > 2000 {
            return Err(Error("一度に解析できる曲は2000曲までです".into()));
        }
        let lib = self.snapshot()?;
        let mut seen = std::collections::HashSet::new();
        let mut missing = Vec::new();
        for id in ids {
            if !seen.insert(id.clone()) {
                continue;
            }
            let t = lib
                .tracks
                .iter()
                .find(|t| &t.id == id)
                .ok_or(Error("曲が見つかりません".into()))?;
            if t.loudness.is_none() || t.dc.is_none() {
                missing.push(t.clone());
            }
        }
        if !missing.is_empty() {
            let root = self.root.clone();
            let measured = self
                .work(move || {
                    missing
                        .into_iter()
                        .map(|mut t| {
                            audio::measure_track(&root.join("sources").join(&t.id), &mut t)?;
                            Ok(t)
                        })
                        .collect::<Result<Vec<_>, String>>()
                })
                .await?;
            self.update(|lib| {
                for t in measured {
                    let old = lib
                        .tracks
                        .iter_mut()
                        .find(|old| old.id == t.id)
                        .ok_or("曲が削除されました")?;
                    if old.trim_start != t.trim_start || old.trim_end != t.trim_end {
                        return Err("曲の解析範囲が変更されました。再実行してください".into());
                    }
                    old.loudness = t.loudness;
                    old.dc = t.dc;
                }
                Ok(())
            })?;
        }
        Ok(self
            .snapshot()?
            .tracks
            .into_iter()
            .filter(|t| seen.contains(&t.id))
            .collect())
    }
    pub(super) fn snapshot(&self) -> Result<Library, Error> {
        self.library
            .lock()
            .map(|v| v.clone())
            .map_err(|_| Error("保存データにアクセスできません".into()))
    }
    pub(super) fn update<T>(
        &self,
        f: impl FnOnce(&mut Library) -> Result<T, String>,
    ) -> Result<T, Error> {
        let mut guard = self
            .library
            .lock()
            .map_err(|_| Error("保存データにアクセスできません".into()))?;
        let mut next = guard.clone();
        let result = f(&mut next)?;
        let bytes = serde_json::to_vec_pretty(&next).map_err(|e| Error(e.to_string()))?;
        let temp = self.root.join("library.tmp");
        fs::write(&temp, bytes).map_err(|e| Error(e.to_string()))?;
        fs::rename(temp, self.root.join("library.json")).map_err(|e| Error(e.to_string()))?;
        *guard = next;
        Ok(result)
    }
    pub(super) async fn work<T: Send + 'static>(
        &self,
        f: impl FnOnce() -> Result<T, String> + Send + 'static,
    ) -> Result<T, Error> {
        let permit = self
            .jobs
            .clone()
            .acquire_owned()
            .await
            .map_err(|e| Error(e.to_string()))?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            f()
        })
        .await
        .map_err(|e| Error(e.to_string()))?
        .map_err(Error)
    }
}

impl App {
    pub(super) fn open() -> Result<Self, Box<dyn std::error::Error>> {
        let root = std::env::var_os("CASSETTE_DATA")
            .map(PathBuf::from)
            .unwrap_or_else(default_data_directory);
        for sub in ["sources", "preview", "exports", "incoming"] {
            fs::create_dir_all(root.join(sub))?;
        }
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("library.lock"))?;
        fs2::FileExt::try_lock_exclusive(&lock).map_err(|_|"Cassette Studio is already using this library. Close the other instance first. / 別のCassette Studioがこのライブラリを使用中です。先に終了してください。")?;
        let file = root.join("library.json");
        let mut initial_library: Library = if file.exists() {
            serde_json::from_slice(&fs::read(&file)?)?
        } else {
            Library::default()
        };
        let before = initial_library.clone();
        for track in &mut initial_library.tracks {
            track.title = audio::repair_legacy_tag(&track.title);
            track.artist = audio::repair_legacy_tag(&track.artist);
        }
        if initial_library != before {
            let backup = root.join("library-before-tag-repair.json");
            if !backup.exists() {
                fs::copy(&file, backup)?;
            }
            let temp = root.join("library.tmp");
            fs::write(&temp, serde_json::to_vec_pretty(&initial_library)?)?;
            fs::rename(temp, &file)?;
        }
        Ok(Self {
            _lock: Arc::new(lock),
            root: root.clone(),
            library: Arc::new(Mutex::new(initial_library)),
            jobs: Arc::new(tokio::sync::Semaphore::new(2)),
        })
    }
}

pub fn default_data_directory() -> PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
        && parent.join("portable.txt").is_file()
    {
        return parent.join("data");
    }
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    if project.join("library.json").is_file() {
        return project;
    }
    directories::ProjectDirs::from("com", "CassetteStudio", "CassetteStudio")
        .map(|p| p.data_local_dir().to_path_buf())
        .unwrap_or(project)
}
