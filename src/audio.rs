use crate::model::*;

/// Recover legacy Japanese tags stored as Latin-1 despite containing CP932 bytes.
/// C1 controls distinguish this case from valid Western or Unicode metadata.
pub fn repair_legacy_tag(text: &str) -> String {
    if !text.chars().any(|c| ('\u{80}'..='\u{9f}').contains(&c)) {
        return text.into();
    }
    let Some(bytes) = text
        .chars()
        .map(|c| u8::try_from(c as u32).ok())
        .collect::<Option<Vec<_>>>()
    else {
        return text.into();
    };
    let Some(decoded) =
        encoding_rs::SHIFT_JIS.decode_without_bom_handling_and_without_replacement(&bytes)
    else {
        return text.into();
    };
    if decoded.chars().any(char::is_control)
        || !decoded.chars().any(|c| {
            ('\u{3040}'..='\u{30ff}').contains(&c) || ('\u{4e00}'..='\u{9fff}').contains(&c)
        })
    {
        return text.into();
    }
    decoded.into_owned()
}

#[cfg(test)]
mod tag_tests {
    use super::repair_legacy_tag;

    #[test]
    fn repairs_legacy_japanese_metadata_without_changing_unicode_or_western_tags() {
        let broken = "\u{93}\u{fa}\u{96}{\u{8c}\u{ea}\u{82}\u{cc}\u{8b}\u{c8}\u{81}\u{99}";
        assert_eq!(repair_legacy_tag(broken), "日本語の曲☆");
        for text in [
            "日本語の曲☆",
            "Björk",
            "Sigur Rós",
            "中文",
            "한글",
            "\u{81}",
            "title\u{fffd}",
        ] {
            assert_eq!(repair_legacy_tag(text), text);
        }
    }
}
use std::{
    fs::{self, File},
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub fn tool(name: &str) -> PathBuf {
    if let Some(p) = std::env::var_os(format!(
        "CASSETTE_{}",
        name.replace('-', "_").to_uppercase()
    )) {
        return p.into();
    }
    let exe = format!("{}{}", name, std::env::consts::EXE_SUFFIX);
    if let Ok(current) = std::env::current_exe()
        && let Some(parent) = current.parent()
    {
        for candidate in [parent.join("tools").join(&exe), parent.join(&exe)] {
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    let local = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tools")
        .join(&exe);
    if local.is_file() { local } else { exe.into() }
}
pub fn command(name: &str) -> Command {
    let c = Command::new(tool(name));
    #[cfg(windows)]
    let c = {
        let mut c = c;
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x08000000);
        c
    };
    c
}
pub fn available(name: &str) -> bool {
    let path = tool(name);
    path.is_file()
        || std::env::var_os("PATH")
            .is_some_and(|value| std::env::split_paths(&value).any(|dir| dir.join(&path).is_file()))
}
pub fn run(c: &mut Command) -> Result<Output, String> {
    let o = c.output().map_err(|e| {
        format!("外部ツールを起動できません: {e}。セットアップ手順を確認してください")
    })?;
    if !o.status.success() {
        let msg = String::from_utf8_lossy(&o.stderr);
        return Err(format!(
            "音声処理に失敗しました: {}",
            msg.chars()
                .rev()
                .take(1500)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        ));
    }
    Ok(o)
}

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
    let filter = format!("silencedetect=noise={}dB:d={}", s.silence_db, s.silence_min);
    let o = run(command("ffmpeg")
        .args(["-hide_banner", "-nostdin", "-i"])
        .arg(path)
        .args(["-map", "0:a:0", "-af", &filter, "-f", "null", "-"]))?;
    let (trim_start, trim_end) = silence_bounds(&String::from_utf8_lossy(&o.stderr), duration);
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
        duration,
        trim_start,
        trim_end,
        manual_gain_db: 0.0,
        loudness: None,
    };
    track.loudness = Some(measure_loudness(path, &track)?);
    Ok(track)
}

// Both measurement and playback use the same stereo downmix and sample rate.
const STEREO: &str = "aresample=44100,aformat=sample_fmts=fltp:channel_layouts=stereo";
pub fn measure_loudness(path: &Path, t: &Track) -> Result<Loudness, String> {
    if t.audible() <= 0.0 {
        return Ok(Loudness {
            integrated_lufs: None,
            true_peak_db: None,
        });
    }
    let filter = format!(
        "{STEREO},atrim=start={}:end={},asetpts=PTS-STARTPTS,loudnorm=I=-18:TP=-1:LRA=50:print_format=json",
        t.trim_start, t.trim_end
    );
    let o = run(command("ffmpeg")
        .args(["-hide_banner", "-nostats", "-nostdin", "-i"])
        .arg(path)
        .args(["-map", "0:a:0", "-vn", "-af", &filter, "-f", "null", "-"]))?;
    parse_loudness(&String::from_utf8_lossy(&o.stderr))
}
fn parse_loudness(log: &str) -> Result<Loudness, String> {
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
    if !s.normalize && t.manual_gain_db == 0.0 {
        return Ok(GainReport {
            gain_db: 0.0,
            limited: false,
            predicted_lufs: t.loudness.as_ref().and_then(|l| l.integrated_lufs),
        });
    }
    let stats = t.loudness.as_ref().ok_or("音量解析が必要です")?;
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
    format!("{}-{:016x}.wav", t.id, hash.finish())
}

fn silence_bounds(log: &str, duration: f64) -> (f64, f64) {
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
            if a <= 0.025 {
                start = v.min(duration);
            }
            if v >= duration - 0.025 {
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

const RATE: u32 = 44100;
const FRAME: u64 = 4;
fn silence(f: &mut File, frames: u64) -> Result<(), String> {
    let zero = [0u8; 65536];
    let mut bytes = frames * FRAME;
    while bytes > 0 {
        let n = bytes.min(zero.len() as u64) as usize;
        f.write_all(&zero[..n]).map_err(|e| e.to_string())?;
        bytes -= n as u64;
    }
    Ok(())
}

/// Stream PCM to disk; memory use is independent of tape length.
pub fn render(
    root: &Path,
    tracks: &[Track],
    entries: &[Entry],
    s: &Settings,
    dest: &Path,
) -> Result<(), String> {
    let mut f = File::create(dest).map_err(|e| e.to_string())?;
    f.write_all(&[0; 44]).map_err(|e| e.to_string())?;
    let raw = dest.with_extension("pcm");
    let result = (|| {
        let mut written = 0u64;
        for (i, e) in entries.iter().enumerate() {
            if i > 0 {
                let frames = (s.gap * RATE as f64).round() as u64;
                silence(&mut f, frames)?;
                written += frames;
            }
            let t = tracks
                .iter()
                .find(|t| t.id == e.track_id)
                .ok_or("曲が見つかりません")?;
            let frames = (e.duration * RATE as f64).round() as u64;
            let correction = gain(t, s)?;
            let mut filter = format!(
                "{STEREO},volume={}dB,atrim=start={}:duration={},asetpts=PTS-STARTPTS",
                correction.gain_db,
                t.trim_start + e.offset,
                e.duration
            );
            if e.fade_in > 0.0 {
                filter.push_str(&format!(",afade=t=in:st=0:d={}", e.fade_in));
            }
            if e.fade_out > 0.0 {
                filter.push_str(&format!(
                    ",afade=t=out:st={}:d={}",
                    (e.duration - e.fade_out).max(0.0),
                    e.fade_out
                ));
            }
            filter.push_str(&format!(",aresample={RATE},apad,atrim=end_sample={frames}"));
            run(command("ffmpeg")
                .args(["-v", "error", "-nostdin", "-y", "-i"])
                .arg(root.join("sources").join(&t.id))
                .args([
                    "-map",
                    "0:a:0",
                    "-vn",
                    "-af",
                    &filter,
                    "-ar",
                    "44100",
                    "-ac",
                    "2",
                    "-c:a",
                    "pcm_s16le",
                    "-f",
                    "s16le",
                ])
                .arg(&raw))?;
            let mut input = File::open(&raw).map_err(|e| e.to_string())?;
            let bytes = std::io::copy(&mut input, &mut f).map_err(|e| e.to_string())?;
            if bytes != frames * FRAME {
                return Err("変換後の音声長が一致しません".into());
            }
            written += frames;
        }
        let total = (s.side_seconds * RATE as f64).round() as u64;
        if crate::model::side_duration(entries, s.gap) > s.side_seconds + 0.001 {
            return Err("面の長さを超えました".into());
        }
        silence(&mut f, total.saturating_sub(written))?;
        // Individual segment rounding can add a few samples. Keep the final
        // file at precisely the requested side length in all cases.
        if written > total {
            f.set_len(44 + total * FRAME).map_err(|e| e.to_string())?;
            f.seek(SeekFrom::End(0)).map_err(|e| e.to_string())?;
        }
        let size = f.stream_position().map_err(|e| e.to_string())? - 44;
        if size > u32::MAX as u64 - 36 {
            return Err("WAVの容量制限を超えました".into());
        }
        f.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        f.write_all(b"RIFF").map_err(|e| e.to_string())?;
        f.write_all(&(size as u32 + 36).to_le_bytes())
            .map_err(|e| e.to_string())?;
        f.write_all(b"WAVEfmt ").map_err(|e| e.to_string())?;
        f.write_all(&16u32.to_le_bytes())
            .map_err(|e| e.to_string())?;
        f.write_all(&1u16.to_le_bytes())
            .map_err(|e| e.to_string())?;
        f.write_all(&2u16.to_le_bytes())
            .map_err(|e| e.to_string())?;
        f.write_all(&RATE.to_le_bytes())
            .map_err(|e| e.to_string())?;
        f.write_all(&(RATE * 4).to_le_bytes())
            .map_err(|e| e.to_string())?;
        f.write_all(&4u16.to_le_bytes())
            .map_err(|e| e.to_string())?;
        f.write_all(&16u16.to_le_bytes())
            .map_err(|e| e.to_string())?;
        f.write_all(b"data").map_err(|e| e.to_string())?;
        f.write_all(&(size as u32).to_le_bytes())
            .map_err(|e| e.to_string())?;
        f.sync_all().map_err(|e| e.to_string())?;
        Ok(())
    })();
    let _ = fs::remove_file(raw);
    if result.is_err() {
        drop(f);
        let _ = fs::remove_file(dest);
    }
    result
}

pub fn preview(root: &Path, t: &Track, s: &Settings) -> Result<(), String> {
    let dest = root.join("preview").join(preview_name(t, s));
    if dest.exists() {
        return Ok(());
    }
    let temp = dest.with_extension(format!("{}.tmp.wav", uuid::Uuid::new_v4()));
    let filter = format!("{STEREO},volume={}dB", gain(t, s)?.gain_db);
    let r = run(command("ffmpeg")
        .args(["-v", "error", "-nostdin", "-y", "-i"])
        .arg(root.join("sources").join(&t.id))
        .args([
            "-map",
            "0:a:0",
            "-vn",
            "-af",
            &filter,
            "-ar",
            "44100",
            "-ac",
            "2",
            "-c:a",
            "pcm_s16le",
        ])
        .arg(&temp));
    match r {
        Ok(_) => fs::rename(&temp, dest).map_err(|e| e.to_string()),
        Err(e) => {
            let _ = fs::remove_file(temp);
            Err(e)
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    fn test_track() -> Track {
        Track {
            id: "test".into(),
            title: String::new(),
            artist: String::new(),
            filename: String::new(),
            source: String::new(),
            duration: 10.0,
            trim_start: 0.0,
            trim_end: 10.0,
            manual_gain_db: 0.0,
            loudness: Some(Loudness {
                integrated_lufs: Some(-24.0),
                true_peak_db: Some(-12.0),
            }),
        }
    }
    #[test]
    fn constant_gain_and_peak_protection() {
        let mut t = test_track();
        let mut s = Settings {
            normalize: true,
            ..Settings::default()
        };
        assert_eq!(gain(&t, &s).unwrap().gain_db, 6.0);
        t.loudness = Some(Loudness {
            integrated_lufs: Some(-50.0),
            true_peak_db: Some(-40.0),
        });
        assert_eq!(gain(&t, &s).unwrap().gain_db, 32.0);
        t = test_track();
        t.manual_gain_db = 2.0;
        assert_eq!(gain(&t, &s).unwrap().gain_db, 8.0);
        t.loudness.as_mut().unwrap().true_peak_db = Some(-3.0);
        let limited = gain(&t, &s).unwrap();
        assert!(limited.limited);
        assert!((limited.gain_db - 1.9).abs() < 1e-9);
        s.normalize = false;
        t.manual_gain_db = 0.0;
        assert_eq!(gain(&t, &s).unwrap().gain_db, 0.0);
        t.manual_gain_db = -3.0;
        assert_eq!(gain(&t, &s).unwrap().gain_db, -3.0);
        t.manual_gain_db = 25.0;
        assert!(gain(&t, &s).is_err());
    }
    #[test]
    fn loudness_silence_and_legacy_data() {
        let l = parse_loudness("log\n{\"input_i\":\"-inf\",\"input_tp\":\"-inf\"}\n").unwrap();
        assert_eq!(l.integrated_lufs, None);
        assert_eq!(l.true_peak_db, None);
        let s: Settings = serde_json::from_str("{\"gap\":3}").unwrap();
        assert!(!s.normalize);
        assert_eq!(s.target_lufs, -18.0);
        let mut t = test_track();
        t.loudness = Some(l);
        assert_eq!(
            gain(
                &t,
                &Settings {
                    normalize: true,
                    ..s
                }
            )
            .unwrap()
            .gain_db,
            0.0
        );
        assert!(parse_loudness("missing output").is_err());
    }
    #[test]
    fn preview_cache_depends_on_gain_and_settings() {
        let mut t = test_track();
        let s = Settings::default();
        let raw = preview_name(&t, &s);
        assert_ne!(
            raw,
            preview_name(
                &t,
                &Settings {
                    normalize: true,
                    ..s.clone()
                }
            )
        );
        t.manual_gain_db = 1.0;
        assert_ne!(raw, preview_name(&t, &s));
    }
    #[test]
    fn only_boundary_silence_is_trimmed() {
        let log = "silence_start: 0\nsilence_end: 2 | silence_duration: 2\nsilence_start: 10\nsilence_end: 15\nsilence_start: 28\nsilence_end: 30";
        assert_eq!(silence_bounds(log, 30.0), (2.0, 28.0));
        assert_eq!(
            silence_bounds("silence_start: 10\nsilence_end: 15", 30.0),
            (0.0, 30.0)
        );
        assert_eq!(
            silence_bounds("silence_start: 0\nsilence_end: 30", 30.0),
            (0.0, 0.0)
        );
    }
}
