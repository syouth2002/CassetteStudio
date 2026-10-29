//! Boundary silence, loudness and per-channel DC measurements.
use super::{
    STEREO,
    processing::dc_filter,
    tools::{command, run},
};
use crate::model::*;
use std::path::Path;
pub(super) fn detect_silence(
    path: &Path,
    duration: f64,
    s: &Settings,
    offset: Option<[f64; 2]>,
) -> Result<(f64, f64), String> {
    let correction = offset
        .map(dc_filter)
        .transpose()?
        .map(|f| format!("{f},"))
        .unwrap_or_default();
    // Count decoded samples in the same pass; container duration can include padding/video.
    let filter = format!(
        "{STEREO},asetpts=PTS-STARTPTS,{correction}silencedetect=noise={}dB:d={},astats=metadata=0:reset=0:measure_perchannel=none:measure_overall=Number_of_samples",
        s.silence_db, s.silence_min
    );
    let o = run(command("ffmpeg")
        .args(["-hide_banner", "-nostdin", "-i"])
        .arg(path)
        .args(["-map", "0:a:0", "-af", &filter, "-f", "null", "-"]))?;
    let log = String::from_utf8_lossy(&o.stderr);
    let decoded_duration = log
        .lines()
        .filter_map(|line| line.split_once("Number of samples: "))
        .filter_map(|(_, value)| value.trim().parse::<u64>().ok())
        .next_back()
        .map(|samples| samples as f64 / 44100.0)
        .filter(|seconds| *seconds > 0.0)
        .unwrap_or(duration);
    Ok(silence_bounds(&log, decoded_duration))
}
pub fn measure_loudness(path: &Path, t: &Track) -> Result<Loudness, String> {
    measure_loudness_with_dc(path, t, None)
}
fn measure_loudness_with_dc(
    path: &Path,
    t: &Track,
    offset: Option<[f64; 2]>,
) -> Result<Loudness, String> {
    if t.audible() <= 0.0 {
        return Ok(Loudness {
            integrated_lufs: None,
            true_peak_db: None,
        });
    }
    let dc = offset
        .map(dc_filter)
        .transpose()?
        .map(|f| format!(",{f}"))
        .unwrap_or_default();
    let filter = format!(
        "{STEREO},atrim=start={}:end={},asetpts=PTS-STARTPTS{dc},loudnorm=I=-18:TP=-1:LRA=50:print_format=json",
        t.trim_start, t.trim_end
    );
    let o = run(command("ffmpeg")
        .args(["-hide_banner", "-nostats", "-nostdin", "-i"])
        .arg(path)
        .args(["-map", "0:a:0", "-vn", "-af", &filter, "-f", "null", "-"]))?;
    parse_loudness(&String::from_utf8_lossy(&o.stderr))
}
pub fn measure_track(path: &Path, t: &mut Track) -> Result<(), String> {
    if t.loudness.is_none() {
        t.loudness = Some(measure_loudness(path, t)?);
    }
    if t.dc.is_none() {
        let offset = measure_dc_range(path, t.trim_start, t.trim_end)?;
        let corrected_loudness = measure_loudness_with_dc(path, t, Some(offset))?;
        t.dc = Some(DcAnalysis {
            offset,
            corrected_loudness,
        });
    }
    Ok(())
}
pub(super) fn measure_dc_range(path: &Path, start: f64, end: f64) -> Result<[f64; 2], String> {
    if end <= start {
        return Ok([0.0; 2]);
    }
    let filter = format!(
        "{STEREO},atrim=start={start}:end={end},astats=metadata=0:reset=0:measure_perchannel=DC_offset:measure_overall=none"
    );
    let o = run(command("ffmpeg")
        .args(["-hide_banner", "-nostats", "-nostdin", "-i"])
        .arg(path)
        .args(["-map", "0:a:0", "-vn", "-af", &filter, "-f", "null", "-"]))?;
    parse_dc(&String::from_utf8_lossy(&o.stderr))
}
pub(super) fn parse_dc(log: &str) -> Result<[f64; 2], String> {
    let values: Vec<f64> = log
        .lines()
        .filter_map(|line| line.split_once("DC offset: ").map(|(_, v)| v.trim()))
        .map(|v| {
            v.parse::<f64>()
                .map_err(|_| "DCオフセットの解析値が不正です".to_string())
        })
        .collect::<Result<_, _>>()?;
    if values.len() != 2 || values.iter().any(|v| !v.is_finite()) {
        return Err("DCオフセットの解析結果を取得できません".into());
    }
    Ok([values[0], values[1]])
}
pub(super) fn parse_loudness(log: &str) -> Result<Loudness, String> {
    let start = log.rfind('{').ok_or("音量の解析結果を取得できません")?;
    let end = log[start..].find('}').ok_or("音量の解析結果が不正です")? + start;
    let json: serde_json::Value =
        serde_json::from_str(&log[start..=end]).map_err(|e| e.to_string())?;
    let number = |key: &str| -> Result<Option<f64>, String> {
        let text = json[key].as_str().ok_or("音量の解析項目がありません")?;
        let value = text.parse::<f64>().map_err(|_| "音量の解析値が不正です")?;
        Ok(value.is_finite().then_some(value))
    };
    Ok(Loudness {
        integrated_lufs: number("input_i")?,
        true_peak_db: number("input_tp")?,
    })
}

pub(super) fn silence_bounds(log: &str, duration: f64) -> (f64, f64) {
    let mut start = 0.0;
    let mut end = duration;
    let mut open = None;
    for line in log.lines() {
        if let Some(v) = line
            .split("silence_start: ")
            .nth(1)
            .and_then(|v| v.split_whitespace().next())
            .and_then(|v| v.parse::<f64>().ok())
        {
            open = Some(v);
        }
        if let Some(v) = line
            .split("silence_end: ")
            .nth(1)
            .and_then(|v| v.split_whitespace().next())
            .and_then(|v| v.parse::<f64>().ok())
            && let Some(a) = open.take()
        {
            if a <= 1.0 / 44100.0 {
                start = v.min(duration);
            }
            if v >= duration - 1.0 / 44100.0 {
                end = a.max(0.0);
            }
        }
    }
    if let Some(a) = open {
        end = a.max(0.0);
    }
    if end <= start {
        return (0.0, 0.0);
    }
    (start, end)
}
