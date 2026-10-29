//! Local media probing and URL ingestion.
use super::{
    analysis::{detect_silence, measure_dc_range, measure_track},
    metadata::repair_legacy_tag,
    tools::{command, run, tool},
};
use crate::model::*;
use std::{fs, path::Path};
pub fn analyze(
    path: &Path,
    id: String,
    filename: String,
    source: String,
    s: &Settings,
) -> Result<Track, String> {
    s.validate()?;
    let o = run(command("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "format=duration:format_tags=title,artist:stream=codec_type,duration",
            "-of",
            "json",
        ])
        .arg(path))?;
    let meta: serde_json::Value = serde_json::from_slice(&o.stdout).map_err(|e| e.to_string())?;
    if meta["streams"].as_array().is_none_or(|a| a.is_empty()) {
        return Err("音声トラックがないファイルです".into());
    }
    let duration = meta["format"]["duration"]
        .as_str()
        .or_else(|| meta["streams"][0]["duration"].as_str())
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.0 && *v <= 86400.0)
        .ok_or("再生時間を取得できないか、24時間を超えるファイルです")?;
    let (mut trim_start, mut trim_end) = detect_silence(path, duration, s, None)?;
    if s.remove_dc {
        let offset = measure_dc_range(path, trim_start, trim_end)?;
        let (start, end) = detect_silence(path, duration, s, Some(offset))?;
        // Preserve genuine digital silence as well as silence hidden by DC.
        trim_start = trim_start.max(start);
        trim_end = trim_end.min(end).max(trim_start);
    }
    let tags = &meta["format"]["tags"];
    let title = tags["title"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(repair_legacy_tag)
        .unwrap_or_else(|| {
            Path::new(&filename)
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });
    let mut track = Track {
        id,
        title,
        artist: repair_legacy_tag(tags["artist"].as_str().unwrap_or("")),
        filename,
        source,
        content_hash: None,
        duration,
        trim_start,
        trim_end,
        manual_gain_db: 0.0,
        loudness: None,
        dc: None,
    };
    measure_track(path, &mut track)?;
    Ok(track)
}

pub fn import_url(root: &Path, input: &str, s: &Settings) -> Result<Track, String> {
    let u = url::Url::parse(input).map_err(|_| "URLが正しくありません")?;
    if !["https", "http"].contains(&u.scheme())
        || u.host_str().is_none()
        || !u.username().is_empty()
        || u.password().is_some()
    {
        return Err("http/httpsの動画URLを入力してください".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let dir = root.join("incoming").join(&id);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let result = (|| {
        let template = dir.join("media.%(ext)s");
        let mut c = command("yt-dlp");
        let node = tool("node");
        c.arg("--js-runtimes").arg(if node.is_file() {
            format!("node:{}", node.display())
        } else {
            "node".into()
        });
        c.args([
            "--no-playlist",
            "--no-progress",
            "--no-warnings",
            "--max-filesize",
            "2G",
            "--socket-timeout",
            "30",
            "--retries",
            "2",
            "-f",
            "bestaudio/best",
            "--write-info-json",
            "-o",
        ])
        .arg(&template);
        let ffmpeg = tool("ffmpeg");
        if ffmpeg.is_file() {
            c.arg("--ffmpeg-location").arg(ffmpeg.parent().unwrap());
        }
        run(c.arg("--").arg(input))?;
        let files = fs::read_dir(&dir)
            .map_err(|e| e.to_string())?
            .filter_map(Result::ok)
            .map(|f| f.path())
            .collect::<Vec<_>>();
        let file = files
            .iter()
            .find(|p| {
                p.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with("media.")
                    && !matches!(
                        p.extension().and_then(|v| v.to_str()),
                        Some("json" | "part" | "ytdl")
                    )
            })
            .ok_or("音声を取得できませんでした")?;
        let info = fs::read(dir.join("media.info.json"))
            .ok()
            .and_then(|v| serde_json::from_slice::<serde_json::Value>(&v).ok());
        let filename = file.file_name().unwrap().to_string_lossy().to_string();
        let mut t = analyze(file, id.clone(), filename, input.into(), s)?;
        if let Some(info) = info {
            if let Some(title) = info["title"].as_str() {
                t.title = title.into();
            }
            t.artist = info["artist"]
                .as_str()
                .or_else(|| info["uploader"].as_str())
                .unwrap_or("")
                .into();
        }
        fs::rename(file, root.join("sources").join(&id)).map_err(|e| e.to_string())?;
        Ok(t)
    })();
    let _ = fs::remove_dir_all(&dir);
    result
}
