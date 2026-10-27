//! DC correction filters, constant gain/peak protection and preview cache identity.
use super::STEREO;
use crate::model::*;
pub(super) fn dc_filter(offset: [f64; 2]) -> Result<String, String> {
    if offset.iter().any(|v| !v.is_finite()) {
        return Err("DCオフセットの解析値が不正です".into());
    }
    Ok(format!(
        "aeval=exprs='val(0)-({})|val(1)-({})':channel_layout=stereo",
        offset[0], offset[1]
    ))
}
pub(super) fn processing_filter(t: &Track, s: &Settings) -> Result<String, String> {
    let dc = if s.remove_dc {
        format!(
            ",{}",
            dc_filter(t.dc.as_ref().ok_or("DCオフセット解析が必要です")?.offset)?
        )
    } else {
        String::new()
    };
    Ok(format!("{STEREO}{dc},volume={}dB", gain(t, s)?.gain_db))
}
#[derive(Debug, serde::Serialize)]
pub struct GainReport {
    pub gain_db: f64,
    pub limited: bool,
    pub predicted_lufs: Option<f64>,
}
pub fn gain(t: &Track, s: &Settings) -> Result<GainReport, String> {
    s.validate()?;
    if !t.manual_gain_db.is_finite() || !(-24.0..=24.0).contains(&t.manual_gain_db) {
        return Err("手動補正は−24〜＋24 dBで指定してください".into());
    }
    if !s.normalize && t.manual_gain_db == 0.0 && !s.remove_dc {
        return Ok(GainReport {
            gain_db: 0.0,
            limited: false,
            predicted_lufs: t.loudness.as_ref().and_then(|l| l.integrated_lufs),
        });
    }
    let stats = if s.remove_dc {
        &t.dc
            .as_ref()
            .ok_or("DCオフセット解析が必要です")?
            .corrected_loudness
    } else {
        t.loudness.as_ref().ok_or("音量解析が必要です")?
    };
    let requested = if s.normalize {
        stats.integrated_lufs.map_or(0.0, |i| s.target_lufs - i)
    } else {
        0.0
    } + t.manual_gain_db;
    // Constant gain preserves dynamics. Reduce gain if the target would exceed
    // the measured true-peak ceiling; 0.1 dB covers measurement/PCM rounding.
    let ceiling = stats
        .true_peak_db
        .map_or(f64::INFINITY, |peak| s.peak_db - 0.1 - peak);
    let gain_db = requested.min(ceiling);
    Ok(GainReport {
        gain_db,
        limited: gain_db < requested - 0.01,
        predicted_lufs: stats.integrated_lufs.map(|i| i + gain_db),
    })
}
pub fn preview_name(t: &Track, s: &Settings) -> String {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    (
        s.normalize,
        s.target_lufs.to_bits(),
        s.peak_db.to_bits(),
        t.manual_gain_db.to_bits(),
        t.trim_start.to_bits(),
        t.trim_end.to_bits(),
    )
        .hash(&mut hash);
    s.remove_dc.hash(&mut hash);
    if s.remove_dc {
        t.dc.as_ref()
            .map(|d| {
                (
                    d.offset[0].to_bits(),
                    d.offset[1].to_bits(),
                    d.corrected_loudness.integrated_lufs.map(f64::to_bits),
                    d.corrected_loudness.true_peak_db.map(f64::to_bits),
                )
            })
            .hash(&mut hash);
    }
    format!("{}-{:016x}.wav", t.id, hash.finish())
}
