use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub filename: String,
    pub source: String,
    pub duration: f64,
    pub trim_start: f64,
    pub trim_end: f64,
    #[serde(default)]
    pub manual_gain_db: f64,
    #[serde(default)]
    pub loudness: Option<Loudness>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Loudness {
    pub integrated_lufs: Option<f64>,
    pub true_peak_db: Option<f64>,
}
impl Track {
    pub fn audible(&self) -> f64 {
        (self.trim_end - self.trim_start).max(0.0)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub side_seconds: f64,
    /// Already included in side_seconds; preserved to edit the nominal length independently.
    pub length_correction: f64,
    pub gap: f64,
    pub fill_threshold: f64,
    pub fade: f64,
    pub continuation: String,
    pub rewind: f64,
    pub silence_db: f64,
    pub silence_min: f64,
    pub normalize: bool,
    pub target_lufs: f64,
    pub peak_db: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            side_seconds: 1800.0,
            length_correction: 0.0,
            gap: 2.0,
            fill_threshold: 30.0,
            fade: 5.0,
            continuation: "restart".into(),
            rewind: 3.0,
            silence_db: -50.0,
            silence_min: 0.2,
            normalize: false,
            target_lufs: -18.0,
            peak_db: -1.0,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<(), String> {
        for (name, value, min, max) in [
            ("片面の長さ", self.side_seconds, 1.0, 7200.0),
            ("長さの補正", self.length_correction, -3600.0, 3600.0),
            ("曲間", self.gap, 0.0, 60.0),
            ("余白の基準", self.fill_threshold, 0.0, 7200.0),
            ("フェード", self.fade, 0.01, 60.0),
            ("巻き戻し", self.rewind, 0.0, 60.0),
            ("無音しきい値", self.silence_db, -100.0, -10.0),
            ("無音継続時間", self.silence_min, 0.01, 10.0),
            ("目標音量", self.target_lufs, -36.0, -8.0),
            ("ピーク上限", self.peak_db, -12.0, -0.1),
        ] {
            if !value.is_finite() || value < min || value > max {
                return Err(format!("{name}の値が範囲外です"));
            }
        }
        if self.continuation != "restart" && self.continuation != "resume" {
            return Err("引き継ぎ方式が不正です".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub track_id: String,
    /// Offset relative to the first audible sample, not the source file.
    pub offset: f64,
    pub duration: f64,
    pub fade_in: f64,
    pub fade_out: f64,
    pub continuation: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub tracks: Vec<String>,
    pub sides: [Vec<Entry>; 2],
    pub settings: Settings,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Library {
    pub tracks: Vec<Track>,
    pub collections: Vec<Collection>,
}

pub fn side_duration(entries: &[Entry], gap: f64) -> f64 {
    entries.iter().map(|e| e.duration).sum::<f64>() + gap * entries.len().saturating_sub(1) as f64
}
