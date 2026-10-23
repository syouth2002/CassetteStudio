mod audio;
mod model;
mod planner;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, Path, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{delete, get, post, put},
};
use model::*;
use serde::Deserialize;
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tokio::io::AsyncWriteExt;
use tower_http::services::ServeDir;

#[derive(Clone)]
struct App {
    _lock: Arc<fs::File>,
    root: PathBuf,
    library: Arc<Mutex<Library>>,
    jobs: Arc<tokio::sync::Semaphore>,
}
struct Error(String);
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
type Api<T> = Result<Json<T>, Error>;
impl App {
    async fn ensure_loudness(&self, ids: &[String]) -> Result<Vec<Track>, Error> {
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
            if t.loudness.is_none() {
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
                            t.loudness = Some(audio::measure_loudness(
                                &root.join("sources").join(&t.id),
                                &t,
                            )?);
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
    fn snapshot(&self) -> Result<Library, Error> {
        self.library
            .lock()
            .map(|v| v.clone())
            .map_err(|_| Error("保存データにアクセスできません".into()))
    }
    fn update<T>(&self, f: impl FnOnce(&mut Library) -> Result<T, String>) -> Result<T, Error> {
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
    async fn work<T: Send + 'static>(
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

async fn same_origin(req: Request<axum::body::Body>, next: Next) -> Response {
    if req.method() != axum::http::Method::GET && req.method() != axum::http::Method::HEAD {
        let headers = req.headers();
        let bad_site =
            headers.get("sec-fetch-site").and_then(|s| s.to_str().ok()) == Some("cross-site");
        let bad_origin = headers
            .get("origin")
            .and_then(|s| s.to_str().ok())
            .is_some_and(|origin| {
                let host = headers
                    .get("host")
                    .and_then(|s| s.to_str().ok())
                    .unwrap_or("");
                origin != format!("http://{host}") && origin != format!("https://{host}")
            });
        if bad_site || bad_origin {
            return (
                StatusCode::FORBIDDEN,
                "別のサイトからの操作は許可されていません",
            )
                .into_response();
        }
    }
    next.run(req).await
}

pub async fn start_server() -> Result<Server, Box<dyn std::error::Error>> {
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
    let app = App {
        _lock: Arc::new(lock),
        root: root.clone(),
        library: Arc::new(Mutex::new(initial_library)),
        jobs: Arc::new(tokio::sync::Semaphore::new(2)),
    };
    let routes = Router::new()
        .route(
            "/favicon.png",
            get(|| async {
                (
                    [("content-type", "image/png")],
                    include_bytes!("../assets/cassette-studio-favicon.png").as_slice(),
                )
            }),
        )
        .route(
            "/",
            get(|| async { Html(include_str!("../web/index.html")) }),
        )
        .route(
            "/app.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../web/app.js"),
                )
            }),
        )
        .route(
            "/style.css",
            get(|| async {
                (
                    [("content-type", "text/css; charset=utf-8")],
                    include_str!("../web/style.css"),
                )
            }),
        )
        .route(
            "/i18n.js",
            get(|| async {
                (
                    [("content-type", "text/javascript; charset=utf-8")],
                    include_str!("../web/i18n.js"),
                )
            }),
        )
        .route(
            "/en.json",
            get(|| async {
                (
                    [("content-type", "application/json; charset=utf-8")],
                    include_str!("../web/en.json"),
                )
            }),
        )
        .route("/api/library", get(library))
        .route("/api/status", get(status))
        .route("/api/import", post(import))
        .route("/api/url", post(import_url))
        .route("/api/collections", post(save_collection))
        .route("/api/collections/{id}", delete(remove_collection))
        .route("/api/plan", post(make_plan))
        .route("/api/fill-cassettes", post(fill_cassettes))
        .route("/api/render", post(render))
        .route("/api/preview/{id}", post(preview))
        .route("/api/reanalyze", put(reanalyze))
        .route("/api/loudness", post(loudness))
        .route("/api/tracks/{id}/gain", put(manual_gain))
        .nest_service("/media", ServeDir::new(root.join("preview")))
        .nest_service("/exports", ServeDir::new(root.join("exports")))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024 * 1024))
        .layer(middleware::from_fn(same_origin))
        .with_state(app);
    let port = std::env::var("CASSETTE_PORT")
        .unwrap_or_else(|_| "3210".into())
        .parse::<u16>()?;
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    println!(
        "Cassette Studio → http://127.0.0.1:{}",
        listener.local_addr()?.port()
    );
    println!("Data: {}\nCtrl+C to stop", root.display());
    let address = listener.local_addr()?;
    let (shutdown, stopped) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let _ = axum::serve(listener, routes)
            .with_graceful_shutdown(async {
                let _ = stopped.await;
            })
            .await;
    });
    Ok(Server {
        url: format!("http://{address}/"),
        root,
        shutdown: Some(shutdown),
        task,
    })
}
pub struct Server {
    pub url: String,
    pub root: PathBuf,
    pub shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    pub task: tokio::task::JoinHandle<()>,
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

async fn library(State(app): State<App>) -> Api<Library> {
    Ok(Json(app.snapshot()?))
}
async fn status(State(app): State<App>) -> Api<serde_json::Value> {
    let data=app.work(||{
        let check=audio::available;
        Ok(serde_json::json!({"ffmpeg":check("ffmpeg"),"ffprobe":check("ffprobe"),"yt_dlp":check("yt-dlp"),"version":env!("CARGO_PKG_VERSION")}))
    }).await?;
    Ok(Json(data))
}

async fn import(State(app): State<App>, mut multi: Multipart) -> Api<serde_json::Value> {
    let mut settings = Settings::default();
    let mut added = vec![];
    let mut errors = vec![];
    while let Some(mut field) = multi.next_field().await.map_err(|e| Error(e.to_string()))? {
        if field.name() == Some("settings") {
            settings = serde_json::from_str(&field.text().await.map_err(|e| Error(e.to_string()))?)
                .map_err(|e| Error(e.to_string()))?;
            settings.validate()?;
            continue;
        }
        if field.name() != Some("files") {
            continue;
        }
        let filename = field
            .file_name()
            .unwrap_or("audio")
            .replace(['\\', '/'], "_");
        let id = uuid::Uuid::new_v4().to_string();
        let path = app.root.join("sources").join(&id);
        let mut f = tokio::fs::File::create(&path)
            .await
            .map_err(|e| Error(e.to_string()))?;
        while let Some(chunk) = field.chunk().await.map_err(|e| Error(e.to_string()))? {
            f.write_all(&chunk)
                .await
                .map_err(|e| Error(e.to_string()))?;
        }
        drop(f);
        let p = path.clone();
        let name = filename.clone();
        let s = settings.clone();
        match app
            .work(move || audio::analyze(&p, id, name, "local".into(), &s))
            .await
        {
            Ok(t) => {
                app.update(|lib| {
                    lib.tracks.push(t.clone());
                    Ok(())
                })?;
                added.push(t);
            }
            Err(e) => {
                let _ = fs::remove_file(path);
                errors.push(format!("{filename}: {}", e.0));
            }
        }
    }
    Ok(Json(serde_json::json!({"tracks":added,"errors":errors})))
}
#[derive(Deserialize)]
struct UrlRequest {
    url: String,
    settings: Settings,
}
async fn import_url(State(app): State<App>, Json(body): Json<UrlRequest>) -> Api<Track> {
    body.settings.validate()?;
    let root = app.root.clone();
    let t = app
        .work(move || audio::import_url(&root, &body.url, &body.settings))
        .await?;
    app.update(|lib| {
        lib.tracks.push(t.clone());
        Ok(())
    })?;
    Ok(Json(t))
}
async fn save_collection(State(app): State<App>, Json(mut c): Json<Collection>) -> Api<Collection> {
    c.settings.validate()?;
    if !["playlist", "cassette"].contains(&c.kind.as_str())
        || c.name.trim().is_empty()
        || c.name.len() > 300
    {
        return Err(Error("名前または種類が不正です".into()));
    }
    if c.id.is_empty() {
        c.id = uuid::Uuid::new_v4().to_string();
    }
    uuid::Uuid::parse_str(&c.id).map_err(|_| Error("IDが不正です".into()))?;
    app.update(|lib| {
        if c.tracks.len() > 5000 || c.sides.iter().any(|v| v.len() > 5000) {
            return Err("曲数の上限を超えました".into());
        }
        if c.tracks
            .iter()
            .chain(c.sides.iter().flatten().map(|e| &e.track_id))
            .any(|id| !lib.tracks.iter().any(|t| &t.id == id))
        {
            return Err("曲が見つかりません".into());
        }
        // Allow overfull draft edits; rendering validates the complete timing.
        let mut validation = c.settings.clone();
        validation.side_seconds = 7200.0;
        if c.kind == "cassette" {
            planner::validate_entries(&c.sides, &lib.tracks, &validation)?;
        }
        if let Some(old) = lib.collections.iter_mut().find(|v| v.id == c.id) {
            *old = c.clone();
        } else {
            lib.collections.push(c.clone());
        }
        Ok(())
    })?;
    Ok(Json(c))
}
async fn remove_collection(
    State(app): State<App>,
    Path(id): Path<String>,
) -> Api<serde_json::Value> {
    app.update(|lib| {
        lib.collections.retain(|c| c.id != id);
        Ok(())
    })?;
    Ok(Json(serde_json::json!({"ok":true})))
}
#[derive(Deserialize)]
struct PlanRequest {
    tracks: Vec<String>,
    settings: Settings,
    optimize: bool,
}
async fn make_plan(State(app): State<App>, Json(body): Json<PlanRequest>) -> Api<[Vec<Entry>; 2]> {
    let lib = app.snapshot()?;
    Ok(Json(
        app.work(move || planner::plan(&lib.tracks, &body.tracks, &body.settings, body.optimize))
            .await?,
    ))
}
#[derive(Deserialize)]
struct RenderRequest {
    sides: [Vec<Entry>; 2],
    settings: Settings,
}
#[derive(Deserialize)]
struct FillRequest {
    tracks: Vec<String>,
    targets: Vec<String>,
}
async fn fill_cassettes(
    State(app): State<App>,
    Json(body): Json<FillRequest>,
) -> Api<planner::BatchPlan> {
    let snapshot = app.snapshot()?;
    let work_snapshot = snapshot.clone();
    let result = app
        .work(move || planner::fill_cassettes(&work_snapshot, &body.tracks, &body.targets))
        .await?;
    app.update(|lib| {
        if *lib != snapshot {
            return Err("配置中にライブラリが変更されました。もう一度実行してください".into());
        }
        for c in &result.collections {
            let old = lib
                .collections
                .iter_mut()
                .find(|old| old.id == c.id)
                .ok_or("配置先が削除されました")?;
            *old = c.clone();
        }
        Ok(())
    })?;
    Ok(Json(result))
}
async fn render(State(app): State<App>, Json(body): Json<RenderRequest>) -> Api<serde_json::Value> {
    let mut lib = app.snapshot()?;
    planner::validate_entries(&body.sides, &lib.tracks, &body.settings)?;
    let ids: Vec<String> = body
        .sides
        .iter()
        .flatten()
        .map(|e| e.track_id.clone())
        .collect();
    if body.settings.normalize
        || lib
            .tracks
            .iter()
            .any(|t| ids.contains(&t.id) && t.manual_gain_db != 0.0)
    {
        app.ensure_loudness(&ids).await?;
        lib = app.snapshot()?;
        planner::validate_entries(&body.sides, &lib.tracks, &body.settings)?;
    }
    let root = app.root.clone();
    let result = app
        .work(move || {
            let id = uuid::Uuid::new_v4().to_string();
            let mut links = vec![];
            for (i, side) in body.sides.iter().enumerate() {
                let name = format!("{id}-{}.wav", if i == 0 { "A" } else { "B" });
                audio::render(
                    &root,
                    &lib.tracks,
                    side,
                    &body.settings,
                    &root.join("exports").join(&name),
                )?;
                links.push(format!("/exports/{name}"));
            }
            let tracks: Vec<&Track> = lib.tracks.iter().filter(|t| ids.contains(&t.id)).collect();
            Ok(serde_json::json!({"urls":links,"tracks":tracks}))
        })
        .await?;
    Ok(Json(result))
}
async fn preview(
    State(app): State<App>,
    Path(id): Path<String>,
    Json(s): Json<Settings>,
) -> Api<serde_json::Value> {
    s.validate()?;
    if s.normalize
        || app
            .snapshot()?
            .tracks
            .iter()
            .any(|t| t.id == id && t.manual_gain_db != 0.0)
    {
        app.ensure_loudness(std::slice::from_ref(&id)).await?;
    }
    let lib = app.snapshot()?;
    let t = lib
        .tracks
        .iter()
        .find(|t| t.id == id)
        .ok_or(Error("曲が見つかりません".into()))?
        .clone();
    let root = app.root.clone();
    let name = audio::preview_name(&t, &s);
    let response_track = t.clone();
    app.work(move || audio::preview(&root, &t, &s)).await?;
    Ok(Json(
        serde_json::json!({"url":format!("/media/{name}"),"track":response_track}),
    ))
}
#[derive(Deserialize)]
struct LoudnessRequest {
    tracks: Vec<String>,
    settings: Settings,
}
async fn loudness(
    State(app): State<App>,
    Json(body): Json<LoudnessRequest>,
) -> Api<serde_json::Value> {
    body.settings.validate()?;
    let tracks = app.ensure_loudness(&body.tracks).await?;
    let reports = tracks
        .iter()
        .map(|t| audio::gain(t, &body.settings).map(|report| (t.id.clone(), report)))
        .collect::<Result<std::collections::BTreeMap<_, _>, _>>()?;
    Ok(Json(serde_json::json!({"tracks":tracks,"reports":reports})))
}
#[derive(Deserialize)]
struct GainRequest {
    gain_db: f64,
}
async fn manual_gain(
    State(app): State<App>,
    Path(id): Path<String>,
    Json(body): Json<GainRequest>,
) -> Api<Track> {
    if !body.gain_db.is_finite() || !(-24.0..=24.0).contains(&body.gain_db) {
        return Err(Error("手動補正は−24〜＋24 dBで指定してください".into()));
    }
    let track = app.update(|lib| {
        let t = lib
            .tracks
            .iter_mut()
            .find(|t| t.id == id)
            .ok_or("曲が見つかりません")?;
        t.manual_gain_db = body.gain_db;
        Ok(t.clone())
    })?;
    Ok(Json(track))
}
async fn reanalyze(State(app): State<App>, Json(s): Json<Settings>) -> Api<Library> {
    s.validate()?;
    let lib = app.snapshot()?;
    if lib
        .collections
        .iter()
        .any(|c| c.kind == "cassette" && !c.sides.iter().all(Vec::is_empty))
    {
        return Err(Error("カセット作成後の一括再解析は既存の再生範囲に影響します。先にカセットを削除してください".into()));
    }
    let root = app.root.clone();
    let tracks = app
        .work(move || {
            lib.tracks
                .into_iter()
                .map(|t| {
                    audio::analyze(
                        &root.join("sources").join(&t.id),
                        t.id,
                        t.filename,
                        t.source,
                        &s,
                    )
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .await?;
    app.update(|lib| {
        for mut t in tracks {
            if let Some(old) = lib.tracks.iter_mut().find(|v| v.id == t.id) {
                t.manual_gain_db = old.manual_gain_db;
                *old = t;
            }
        }
        Ok(())
    })?;
    Ok(Json(app.snapshot()?))
}
