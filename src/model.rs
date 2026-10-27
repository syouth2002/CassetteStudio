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
    #[serde(default)]
    pub dc: Option<DcAnalysis>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Loudness {
    pub integrated_lufs: Option<f64>,
    pub true_peak_db: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DcAnalysis {
    /// Mean sample values after the common stereo downmix, over the trimmed track.
    pub offset: [f64; 2],
    pub corrected_loudness: Loudness,
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
    /// Silence reserved at the beginning of both cassette sides.
    pub lead_in: f64,
    /// None retains the existing 16-bit / 44.1 kHz output behavior.
    pub wav_sample_rate: Option<u32>,
    pub fill_threshold: f64,
    pub fade: f64,
    pub continuation: String,
    pub rewind: f64,
    pub silence_db: f64,
    pub silence_min: f64,
    pub normalize: bool,
    pub remove_dc: bool,
    pub target_lufs: f64,
    pub peak_db: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            side_seconds: 1800.0,
            length_correction: 0.0,
            gap: 2.0,
            lead_in: 0.0,
            wav_sample_rate: None,
            fill_threshold: 30.0,
            fade: 5.0,
            continuation: "restart".into(),
            rewind: 3.0,
            silence_db: -50.0,
            silence_min: 0.2,
            normalize: false,
            remove_dc: false,
            target_lufs: -18.0,
            peak_db: -1.0,
        }
    }
}
impl Settings {
    pub fn content_seconds(&self) -> f64 {
        self.side_seconds - self.lead_in
    }

    pub fn output_sample_rate(&self) -> u32 {
        self.wav_sample_rate.unwrap_or(44100)
    }

    pub fn validate(&self) -> Result<(), String> {
        for (name, value, min, max) in [
            ("片面の長さ", self.side_seconds, 1.0, 7200.0),
            ("長さの補正", self.length_correction, -3600.0, 3600.0),
            ("曲間", self.gap, 0.0, 60.0),
            ("先頭の無音", self.lead_in, 0.0, 7200.0),
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
        if self.lead_in > self.side_seconds {
            return Err("先頭の無音が片面の長さを超えています".into());
        }
        if (self.lead_in * 2.0 - (self.lead_in * 2.0).round()).abs() > 1e-8 {
            return Err("先頭の無音は0.5秒刻みで指定してください".into());
        }
        if !matches!(self.wav_sample_rate, None | Some(44100 | 48000)) {
            return Err("WAVのサンプリングレートは未設定・44100・48000から選択してください".into());
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

#[cfg(test)]
mod settings_tests {
    use super::*;

    #[test]
    fn legacy_settings_and_dat_formats() {
        let mut s: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(s.lead_in, 0.0);
        assert_eq!(s.wav_sample_rate, None);
        assert_eq!(s.output_sample_rate(), 44100);
        for rate in [44100, 48000] {
            s.wav_sample_rate = Some(rate);
            assert!(s.validate().is_ok());
            assert_eq!(s.output_sample_rate(), rate);
        }
        s.wav_sample_rate = Some(96000);
        assert!(s.validate().is_err());
    }

    #[test]
    fn lead_in_reserves_capacity_and_uses_half_seconds() {
        let mut s = Settings {
            side_seconds: 10.0,
            lead_in: 2.5,
            ..Settings::default()
        };
        assert_eq!(s.content_seconds(), 7.5);
        assert!(s.validate().is_ok());
        for invalid in [-0.5, 0.25, 10.5, f64::NAN] {
            s.lead_in = invalid;
            assert!(s.validate().is_err());
        }
    }
}
