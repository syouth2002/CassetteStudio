//! Lightweight source inspection; opening properties never runs audio analysis.
use super::{
    metadata::repair_legacy_tag,
    tools::{command, run},
};
use crate::model::Track;
use serde_json::{Value, json};
use std::{fs, path::Path};
fn number(v: &Value) -> Option<u64> {
    v.as_u64()
        .or_else(|| v.as_str()?.parse().ok())
        .filter(|n| *n > 0)
}
fn container_name(format: &Value) -> String {
    let raw = format["format_name"].as_str().unwrap_or("");
    if raw.split(',').any(|n| n == "mov") {
        let brand = format["tags"]["major_brand"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if brand.starts_with("m4a") {
            "M4A".into()
        } else if brand.starts_with("m4b") {
            "M4B".into()
        } else if brand == "qt" {
            "MOV".into()
        } else if brand.starts_with("3gp") {
            "3GP".into()
        } else if brand.starts_with("3g2") {
            "3G2".into()
        } else if brand == "mjp2" || brand == "mj2s" {
            "MJ2".into()
        } else if brand.starts_with("iso") || brand.starts_with("mp4") || brand == "dash" {
            "MP4".into()
        } else {
            "MP4 / MOV".into()
        }
    } else if raw == "matroska,webm" {
        "Matroska / WebM".into()
    } else {
        raw.to_uppercase()
    }
}
pub fn properties(root: &Path, t: &Track) -> Result<Value, String> {
    let path = root.join("sources").join(&t.id);
    let meta = fs::metadata(&path).map_err(|e| e.to_string())?;
    let out = run(command("ffprobe")
        .args([
            "-v",
            "error",
            "-show_streams",
            "-show_format",
            "-of",
            "json",
        ])
        .arg(&path))?;
    let probe: Value = serde_json::from_slice(&out.stdout).map_err(|e| e.to_string())?;
    let streams = probe["streams"].as_array().ok_or("Streams not found")?;
    let stream = streams
        .iter()
        .find(|s| s["codec_type"] == "audio")
        .ok_or("Audio stream not found")?;
    let has_video = streams.iter().any(|s| {
        s["codec_type"] == "video" && s["disposition"]["attached_pic"].as_u64() != Some(1)
    });
    let has_cover_art = streams.iter().any(|s| {
        s["codec_type"] == "video" && s["disposition"]["attached_pic"].as_u64() == Some(1)
    });
    let format = &probe["format"];
    let codec = stream["codec_name"].as_str().unwrap_or("");
    let lossy = matches!(
        codec,
        "mp3"
            | "mp2"
            | "aac"
            | "opus"
            | "vorbis"
            | "ac3"
            | "eac3"
            | "wmav1"
            | "wmav2"
            | "wmavoice"
            | "amr_nb"
            | "amr_wb"
    );
    let bits = if lossy {
        None
    } else {
        number(&stream["bits_per_raw_sample"]).or_else(|| number(&stream["bits_per_sample"]))
    };
    let bitrate = number(&stream["bit_rate"]).or_else(|| {
        (number(&format["nb_streams"]) == Some(1))
            .then(|| number(&format["bit_rate"]))
            .flatten()
    });
    let mut tags = serde_json::Map::new();
    for source in [&format["tags"], &stream["tags"]] {
        if let Some(values) = source.as_object() {
            for (k, v) in values {
                if let Some(text) = v.as_str() {
                    tags.insert(k.to_lowercase(), json!(repair_legacy_tag(text)));
                }
            }
        }
    }
    Ok(
        json!({"track":t,"format":format["format_name"],"container_name":container_name(format),"has_video":has_video,"has_cover_art":has_cover_art,"codec":codec,"codec_description":stream["codec_long_name"],"sample_rate":number(&stream["sample_rate"]),"bits":bits,"bit_depth_applicable":!lossy,"float_pcm":codec.starts_with("pcm_f"),"channels":number(&stream["channels"]),"channel_layout":stream["channel_layout"],"bit_rate":bitrate,"size":meta.len(),"stored_path":path.to_string_lossy(),"tags":tags}),
    )
}
