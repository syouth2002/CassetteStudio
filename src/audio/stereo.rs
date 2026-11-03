//! Conservative source classification and mono-compatible spectral widening.
use super::tools::{command, run};
use crate::model::Track;
use std::path::Path;

pub fn detect(path: &Path) -> Result<bool, String> {
    let output = run(command("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=channels",
            "-of",
            "json",
        ])
        .arg(path))?;
    let probe: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    match probe["streams"][0]["channels"].as_u64() {
        Some(1) => return Ok(true),
        Some(2) => {}
        Some(_) => return Ok(false),
        None => return Err("チャンネル数を取得できません".into()),
    }
    // Examine the entire first audio stream at its native sample rate, not a sample
    // or the stereo downmix of multichannel media. Silence alone is inconclusive.
    let output = run(command("ffmpeg").args(["-hide_banner", "-nostats", "-nostdin", "-i"]).arg(path).args([
        "-map", "0:a:0", "-vn", "-af",
        "aformat=sample_fmts=dblp,pan=stereo|c0=0.5*c0+0.5*c1|c1=0.5*c0-0.5*c1,astats=metadata=0:reset=0:measure_perchannel=RMS_level:measure_overall=none",
        "-f", "null", "-",
    ]))?;
    let log = String::from_utf8_lossy(&output.stderr);
    let levels: Vec<f64> = log
        .lines()
        .filter_map(|l| l.split_once("RMS level dB: "))
        .filter_map(|(_, v)| v.trim().parse().ok())
        .collect();
    if levels.len() != 2 {
        return Err("モノラル判定に失敗しました".into());
    }
    Ok(levels[0].is_finite() && levels[1] <= levels[0] - 80.0)
}

pub(super) fn filter(track: &Track) -> Result<String, String> {
    let amount = match track.pseudo_stereo {
        0 => return Ok(String::new()),
        1 => 0.15,
        2 => 0.25,
        3 => 0.4,
        _ => return Err("疑似ステレオの設定が不正です".into()),
    };
    if track.mono != Some(true) {
        return Err("疑似ステレオ化はモノラルの曲だけ選択できます".into());
    }
    // FFmpeg allpass/pan: https://ffmpeg.org/ffmpeg-filters.html#allpass
    // P is an all-pass phase-shaped copy of M. L=M+a(P-M), R=M-a(P-M).
    // (L+R)/2 is exactly M: the added side cancels during mono playback.
    // The filter has no block delay; low frequencies remain predominantly central.
    Ok(format!(
        ",pan=stereo|c0=0.5*c0+0.5*c1|c1=0.5*c0+0.5*c1,allpass=f=700:t=q:w=0.7:c=FR,pan=stereo|c0={}*c0+{amount}*c1|c1={}*c0-{amount}*c1",
        1.0 - amount,
        1.0 + amount
    ))
}
