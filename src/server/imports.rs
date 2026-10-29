//! Content identity, legacy-file lookup, and cleanup of uncommitted uploads.
use super::state::{App, Error};
use crate::model::Track;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub(super) fn file_identity(path: &Path) -> Result<(String, u64), String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 65536];
    let mut size = 0;
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
        size += n as u64;
    }
    Ok((format!("sha256:{:x}", digest.finalize()), size))
}

impl App {
    /// Hash legacy sources only when their size matches, and persist their identities once.
    /// Callers hold the import mutex until lookup and registration have both completed.
    pub(super) async fn duplicate_file(
        &self,
        hash: &str,
        size: u64,
    ) -> Result<Option<Track>, Error> {
        let lib = self.snapshot()?;
        if let Some(t) = lib
            .tracks
            .iter()
            .find(|t| t.content_hash.as_deref() == Some(hash))
        {
            return Ok(Some(t.clone()));
        }
        let root = self.root.clone();
        let missing: Vec<_> = lib
            .tracks
            .into_iter()
            .filter(|t| t.content_hash.is_none())
            .collect();
        if missing.is_empty() {
            return Ok(None);
        }
        let measured = self
            .work(move || {
                let mut hashes = Vec::new();
                for track in missing {
                    let path = root.join("sources").join(&track.id);
                    if fs::metadata(&path).is_ok_and(|meta| meta.is_file() && meta.len() == size) {
                        let (hash, _) = file_identity(&path)?;
                        hashes.push((track.id, hash));
                    }
                }
                Ok(hashes)
            })
            .await?;
        if !measured.is_empty() {
            self.update(|lib| {
                for (id, hash) in measured {
                    if let Some(t) = lib.tracks.iter_mut().find(|t| t.id == id)
                        && t.content_hash.is_none()
                    {
                        t.content_hash = Some(hash);
                    }
                }
                Ok(())
            })?;
        }
        Ok(self
            .snapshot()?
            .tracks
            .into_iter()
            .find(|t| t.content_hash.as_deref() == Some(hash)))
    }
}

/// Drop removes only this request's newly created file, never a registered source.
pub(super) struct PendingSource {
    pub(super) path: PathBuf,
    committed: bool,
}
impl PendingSource {
    pub(super) fn new(path: PathBuf) -> Self {
        Self {
            path,
            committed: false,
        }
    }
    pub(super) fn move_to(&mut self, destination: PathBuf) -> Result<(), String> {
        fs::rename(&self.path, &destination).map_err(|e| e.to_string())?;
        self.path = destination;
        Ok(())
    }
    pub(super) fn commit(&mut self) {
        self.committed = true;
    }
    pub(super) fn reuse(&mut self, root: &Path, existing: &Track) -> Result<(), String> {
        let destination = root.join("sources").join(&existing.id);
        if !destination.is_file() {
            self.move_to(destination)?;
            self.commit();
        }
        Ok(())
    }
}
impl Drop for PendingSource {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}
