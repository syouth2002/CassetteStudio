//! Cassette WAV assembly and whole-track preview generation.
use super::{
    processing::{preview_name, processing_filter},
    tools::{command, run},
};
use crate::model::*;
use std::{
    fs::{self, File},
    io::{Seek, SeekFrom, Write},
    path::Path,
};
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
    s.validate()?;
    let rate = s.output_sample_rate();
    let rate_arg = rate.to_string();
    let mut f = File::create(dest).map_err(|e| e.to_string())?;
    f.write_all(&[0; 44]).map_err(|e| e.to_string())?;
    let raw = dest.with_extension("pcm");
    let result = (|| {
        let mut written = (s.lead_in * rate as f64).round() as u64;
        silence(&mut f, written)?;
        for (i, e) in entries.iter().enumerate() {
            if i > 0 {
                let frames = (s.gap * rate as f64).round() as u64;
                silence(&mut f, frames)?;
                written += frames;
            }
            let t = tracks
                .iter()
                .find(|t| t.id == e.track_id)
                .ok_or("曲が見つかりません")?;
            let frames = (e.duration * rate as f64).round() as u64;
            let mut filter = format!(
                "{},atrim=start={}:duration={},asetpts=PTS-STARTPTS",
                processing_filter(t, s)?,
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
            filter.push_str(&format!(",aresample={rate},apad,atrim=end_sample={frames}"));
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
                    &rate_arg,
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
        let total = (s.side_seconds * rate as f64).round() as u64;
        if crate::model::side_duration(entries, s.gap) > s.content_seconds() + 0.001 {
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
        f.write_all(&rate.to_le_bytes())
            .map_err(|e| e.to_string())?;
        f.write_all(&(rate * 4).to_le_bytes())
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
    let filter = processing_filter(t, s)?;
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
