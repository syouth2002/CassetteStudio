//! HTTP request validation and application operations.
use super::state::{Api, App, Error};
use crate::{audio, model::*, planner};
use axum::{
    Json, Router,
    extract::{Multipart, Path, State},
    routing::{delete, get, post, put},
};
use serde::Deserialize;
use std::fs;
use tokio::io::AsyncWriteExt;
use tower_http::services::ServeDir;

pub(super) fn routes(root: &std::path::Path) -> Router<App> {
    Router::new()
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
        || body.settings.remove_dc
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
        || s.remove_dc
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
