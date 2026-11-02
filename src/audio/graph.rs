//! Cached EBU R128 time-series measurements of the selected playback range.
use super::{STEREO, processing::processing_filter, tools::command};
use crate::model::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, BufReader},
    path::Path,
    process::Stdio,
};
#[derive(Serialize, Deserialize)]
pub struct Series {
    pub points: Vec<Point>,
    pub integrated_lufs: Option<f64>,
    pub true_peak_db: Option<f64>,
}
#[derive(Serialize, Deserialize)]
pub struct Point {
    pub time: f64,
    pub momentary: Option<f64>,
    pub short_term: Option<f64>,
}
#[derive(Serialize, Deserialize)]
pub struct Marker {
    pub time: f64,
    pub end: f64,
    pub title: String,
}
#[derive(Serialize, Deserialize)]
pub struct Graph {
    #[serde(default)]
    pub markers: Vec<Marker>,
    pub duration: f64,
    pub original: Series,
    pub processed: Series,
}
fn value(line: &str, key: &str) -> Option<f64> {
    line.split_once(key)?
        .1
        .split_whitespace()
        .next()?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}
fn measure(path: &Path, filter: &str) -> Result<Series, String> {
    let mut child = command("ffmpeg")
        .args(["-hide_banner", "-nostats", "-nostdin", "-i"])
        .arg(path)
        .args([
            "-map",
            "0:a:0",
            "-vn",
            "-af",
            &format!("{filter},ebur128=peak=true"),
            "-f",
            "null",
            "-",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    let mut result = Series {
        points: vec![],
        integrated_lufs: None,
        true_peak_db: None,
    };
    let mut error_tail = std::collections::VecDeque::new();
    let mut last_bucket = -1_i64;
    for line in BufReader::new(child.stderr.take().ok_or("Audio process unavailable")?).lines() {
        let line = match line {
            Ok(line) => line,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
        };
        if line.contains(" M:")
            && let Some(time) = value(&line, " t:")
        {
            let bucket = (time * 2.0).floor() as i64;
            if bucket != last_bucket {
                result.points.push(Point {
                    time,
                    momentary: value(&line, " M:").filter(|v| time >= 0.399 && *v > -100.0),
                    short_term: value(&line, " S:").filter(|v| time >= 2.999 && *v > -100.0),
                });
                last_bucket = bucket;
            }
        }
        if line.trim_start().starts_with("I:") {
            result.integrated_lufs = value(&line, "I:").filter(|v| *v > -70.0);
        }
        if line.trim_start().starts_with("Peak:") {
            result.true_peak_db = value(&line, "Peak:");
        }
        if error_tail.len() == 16 {
            error_tail.pop_front();
        }
        error_tail.push_back(line.chars().take(500).collect::<String>());
    }
    if !child.wait().map_err(|e| e.to_string())?.success() {
        return Err(format!(
            "LUFS analysis failed: {}",
            error_tail.into_iter().collect::<Vec<_>>().join("\n")
        ));
    }
    Ok(result)
}
pub fn graph(root: &Path, t: &Track, s: &Settings, entry: Option<&Entry>) -> Result<Graph, String> {
    s.validate()?;
    let (offset, duration, fade_in, fade_out) = entry
        .map(|e| (e.offset, e.duration, e.fade_in, e.fade_out))
        .unwrap_or((0.0, t.audible(), 0.0, 0.0));
    if [offset, duration, fade_in, fade_out]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.0)
        || duration <= 0.0
        || duration > 86400.0
        || offset + duration > t.audible() + 0.001
        || fade_in > duration
        || fade_out > duration
    {
        return Err("Invalid playback range (maximum 24 hours)".into());
    }
    let source = root.join("sources").join(&t.id);
    let meta = fs::metadata(&source).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    hash.update(
        serde_json::to_vec(&(3, t, s, entry, meta.len(), meta.modified().ok()))
            .map_err(|e| e.to_string())?,
    );
    let folder = root.join("loudness-graphs");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let cache = folder.join(format!("{:x}.json", hash.finalize()));
    if let Ok(bytes) = fs::read(&cache)
        && let Ok(data) = serde_json::from_slice(&bytes)
    {
        return Ok(data);
    }
    let range = format!(
        "atrim=start={}:duration={duration},asetpts=PTS-STARTPTS",
        t.trim_start + offset
    );
    let original = measure(&source, &format!("{STEREO},{range}"))?;
    let mut processed = format!("{},{range}", processing_filter(t, s)?);
    if fade_in > 0.0 {
        processed.push_str(&format!(",afade=t=in:st=0:d={fade_in}"));
    }
    if fade_out > 0.0 {
        processed.push_str(&format!(
            ",afade=t=out:st={}:d={fade_out}",
            (duration - fade_out).max(0.0)
        ));
    }
    // Match the WAV output format, including sample-rate conversion and 16-bit quantization.
    processed.push_str(&format!(
        ",aresample={},aformat=sample_fmts=s16:channel_layouts=stereo",
        s.output_sample_rate()
    ));
    let data = Graph {
        markers: vec![],
        duration,
        original,
        processed: measure(&source, &processed)?,
    };
    let temp = cache.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    fs::write(&temp, serde_json::to_vec(&data).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    if let Err(e) = fs::rename(&temp, &cache) {
        let _ = fs::remove_file(&temp);
        if !cache.exists() {
            return Err(e.to_string());
        }
    }
    Ok(data)
}

/// Render one side through the actual WAV pipeline so windows span gaps and track boundaries.
pub fn side_graph(
    root: &Path,
    tracks: &[Track],
    entries: &[Entry],
    s: &Settings,
) -> Result<Graph, String> {
    crate::planner::validate_entries(&[entries.to_vec(), vec![]], tracks, s)?;
    let mut hash = Sha256::new();
    hash.update(serde_json::to_vec(&("side-v1", entries, s)).map_err(|e| e.to_string())?);
    let mut used = Vec::new();
    let mut markers = Vec::new();
    let mut position = s.lead_in;
    for (i, e) in entries.iter().enumerate() {
        let t = tracks
            .iter()
            .find(|t| t.id == e.track_id)
            .ok_or("曲が見つかりません")?;
        let meta = fs::metadata(root.join("sources").join(&t.id)).map_err(|e| e.to_string())?;
        hash.update(
            serde_json::to_vec(&(t, meta.len(), meta.modified().ok()))
                .map_err(|e| e.to_string())?,
        );
        if i > 0 {
            position += s.gap;
        }
        markers.push(Marker {
            time: position,
            end: position + e.duration,
            title: t.title.clone(),
        });
        position += e.duration;
        if !used.iter().any(|old: &Track| old.id == t.id) {
            used.push(t.clone());
        }
    }
    let folder = root.join("loudness-graphs");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let cache = folder.join(format!("side-{:x}.json", hash.finalize()));
    if let Ok(bytes) = fs::read(&cache)
        && let Ok(data) = serde_json::from_slice(&bytes)
    {
        return Ok(data);
    }
    let temp = folder.join(format!("{}.wav", uuid::Uuid::new_v4()));
    let result: Result<Graph, String> = (|| {
        let mut raw = s.clone();
        raw.normalize = false;
        raw.remove_dc = false;
        let mut originals = used.clone();
        for t in &mut originals {
            t.manual_gain_db = 0.0;
        }
        super::render::render(root, &originals, entries, &raw, &temp)?;
        let original = measure(&temp, "anull")?;
        super::render::render(root, &used, entries, s, &temp)?;
        Ok(Graph {
            duration: s.side_seconds,
            markers,
            original,
            processed: measure(&temp, "anull")?,
        })
    })();
    let _ = fs::remove_file(&temp);
    let data = result?;
    let temp_cache = cache.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    fs::write(
        &temp_cache,
        serde_json::to_vec(&data).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if let Err(e) = fs::rename(&temp_cache, &cache) {
        let _ = fs::remove_file(temp_cache);
        if !cache.exists() {
            return Err(e.to_string());
        }
    }
    Ok(data)
}
