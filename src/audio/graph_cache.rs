//! Independent caches for reusable original and processed LUFS time series.
use super::graph::Series;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn hash(value: &impl Serialize) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).map_err(|e| e.to_string())?)
    ))
}
pub(super) fn path(root: &Path, kind: &str, identity: &impl Serialize) -> Result<PathBuf, String> {
    Ok(root
        .join("loudness-series")
        .join(format!("{kind}-{}.json", hash(identity)?)))
}
pub(super) fn load(path: &Path) -> Option<Series> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}
pub(super) fn store(path: &Path, value: &impl Serialize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    fs::write(
        &temporary,
        serde_json::to_vec(value).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if fs::rename(&temporary, path).is_err() {
        // Windows does not replace an existing file with rename. Only cache
        // files are replaced here; source audio and library data are untouched.
        let _ = fs::remove_file(path);
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::remove_file(temporary);
            return Err(error.to_string());
        }
    }
    Ok(())
}
pub(super) fn seed(path: &Path, value: &Series) {
    if load(path).is_none() {
        let _ = store(path, value);
    }
}
pub(super) fn resolve(
    path: &Path,
    measure: impl FnOnce() -> Result<Series, String>,
) -> Result<Series, String> {
    if let Some(value) = load(path) {
        return Ok(value);
    }
    let value = measure()?;
    store(path, &value)?;
    Ok(value)
}
