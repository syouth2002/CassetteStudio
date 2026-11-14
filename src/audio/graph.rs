//! Cached EBU R128 time-series measurements of the selected playback range.
use super::{STEREO, graph_cache as cache, processing::processing_filter, tools::command};
use crate::model::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::Stdio,
};
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct Series {
    pub points: Vec<Point>,
    pub integrated_lufs: Option<f64>,
    pub true_peak_db: Option<f64>,
}
#[derive(Clone, PartialEq, Serialize, Deserialize)]
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

fn cached_graph(path: &Path, legacy: &Path) -> Option<Graph> {
    for candidate in [path, legacy] {
        if let Ok(bytes) = fs::read(candidate)
            && let Ok(graph) = serde_json::from_slice::<Graph>(&bytes)
        {
            if candidate != path {
                let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
                if fs::write(&temporary, &bytes).is_ok() {
                    let _ = fs::rename(&temporary, path);
                }
                let _ = fs::remove_file(temporary);
            }
            return Some(graph);
        }
    }
    None
}

#[cfg(test)]
#[test]
fn cached_graph_promotes_legacy_results_without_measuring_audio() {
    let folder =
        std::env::temp_dir().join(format!("cassette-graph-cache-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&folder).unwrap();
    let legacy = folder.join("legacy.json");
    let current = folder.join("current.json");
    let bytes=br#"{"duration":1,"original":{"points":[],"integrated_lufs":-123,"true_peak_db":null},"processed":{"points":[],"integrated_lufs":-122,"true_peak_db":null}}"#;
    fs::write(&legacy, bytes).unwrap();
    assert_eq!(
        cached_graph(&current, &legacy)
            .unwrap()
            .original
            .integrated_lufs,
        Some(-123.)
    );
    assert_eq!(fs::read(&current).unwrap(), bytes);
    fs::remove_file(&legacy).unwrap();
    assert_eq!(
        cached_graph(&current, &legacy)
            .unwrap()
            .processed
            .integrated_lufs,
        Some(-122.)
    );
    fs::remove_file(&current).unwrap();
    fs::remove_dir(folder).unwrap();
}
pub(super) fn save_source_series(path: &Path, t: &Track, log: &str) -> Result<(), String> {
    let Some(parent) = path.parent() else {
        return Ok(());
    };
    if !matches!(
        parent.file_name().and_then(|n| n.to_str()),
        Some("sources" | "incoming")
    ) {
        return Ok(());
    }
    let Some(root) = parent.parent() else {
        return Ok(());
    };
    let meta = fs::metadata(path).map_err(|e| e.to_string())?;
    let raw_filter = format!(
        "{STEREO},atrim=start={}:duration={},asetpts=PTS-STARTPTS",
        t.trim_start,
        t.audible()
    );
    let target = cache::path(
        root,
        "source",
        &(
            "track-series-v1",
            (&t.id, meta.len(), meta.modified().ok()),
            raw_filter,
        ),
    )?;
    let mut points = vec![];
    let mut last_bucket = -1_i64;
    for line in log.lines().filter(|line| line.contains(" M:")) {
        if let Some(time) = value(line, " t:") {
            let bucket = (time * 2.).floor() as i64;
            if bucket != last_bucket {
                points.push(Point {
                    time,
                    momentary: value(line, " M:").filter(|v| time >= 0.399 && *v > -100.),
                    short_term: value(line, " S:").filter(|v| time >= 2.999 && *v > -100.),
                });
                last_bucket = bucket;
            }
        }
    }
    cache::store(
        &target,
        &Series {
            points,
            integrated_lufs: log
                .lines()
                .filter(|line| line.trim_start().starts_with("I:"))
                .filter_map(|line| value(line, "I:"))
                .rfind(|v| *v > -70.),
            true_peak_db: log
                .lines()
                .filter(|line| line.trim_start().starts_with("Peak:"))
                .filter_map(|line| value(line, "Peak:"))
                .next_back(),
        },
    )
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
type PlaybackRange = (f64, f64, f64, f64);

fn pair_path(
    folder: &Path,
    t: &Track,
    s: &Settings,
    range: PlaybackRange,
    meta: &fs::Metadata,
) -> Result<PathBuf, String> {
    let (offset, duration, fade_in, fade_out) = range;
    let identity = (
        4,
        &t.id,
        t.trim_start,
        offset,
        duration,
        fade_in,
        fade_out,
        processing_filter(t, s)?,
        s.output_sample_rate(),
        meta.len(),
        meta.modified().ok(),
    );
    Ok(folder.join(format!("{}.json", cache::hash(&identity)?)))
}
fn legacy_path(
    folder: &Path,
    t: &Track,
    s: &Settings,
    entry: Option<&Entry>,
    meta: &fs::Metadata,
) -> Result<PathBuf, String> {
    Ok(folder.join(format!(
        "{}.json",
        cache::hash(&(3, t, s, entry, meta.len(), meta.modified().ok()))?
    )))
}
// Previous releases saved pairs rather than independent series. Reuse the
// original half of an existing uncorrected graph without decoding audio again.
fn existing_original(
    folder: &Path,
    t: &Track,
    s: &Settings,
    range: PlaybackRange,
    meta: &fs::Metadata,
) -> Option<Series> {
    let (offset, duration, _, _) = range;
    let entry = Entry {
        track_id: t.id.clone(),
        offset,
        duration,
        fade_in: 0.,
        fade_out: 0.,
        continuation: false,
    };
    let legacy_entry = if offset == 0. && duration == t.audible() {
        None
    } else {
        Some(&entry)
    };
    let candidates = [
        Settings::default(),
        Settings {
            wav_sample_rate: Some(48000),
            ..Settings::default()
        },
        Settings {
            normalize: false,
            remove_dc: false,
            ..s.clone()
        },
    ];
    for candidate in candidates {
        let Some(pair) = pair_path(folder, t, &candidate, (offset, duration, 0., 0.), meta).ok()
        else {
            continue;
        };
        let Some(legacy) = legacy_path(folder, t, &candidate, legacy_entry, meta).ok() else {
            continue;
        };
        if let Some(data) = cached_graph(&pair, &legacy) {
            return Some(data.original);
        }
    }
    None
}

/// Prepare both the configured output and the normalized library comparison.
/// Their original curve is shared; only a missing processed curve is measured.
pub fn prepare_track_graphs(root: &Path, t: &Track, s: &Settings) -> Result<(), String> {
    if t.audible() <= 0. {
        return Ok(());
    }
    graph(root, t, s, None)?;
    if !s.normalize {
        graph(root, t, &s.loudness_preview(), None)?;
    }
    Ok(())
}

pub fn graph(root: &Path, t: &Track, s: &Settings, entry: Option<&Entry>) -> Result<Graph, String> {
    graph_with_measure(root, t, s, entry, &mut measure)
}
fn graph_with_measure(
    root: &Path,
    t: &Track,
    s: &Settings,
    entry: Option<&Entry>,
    measure: &mut impl FnMut(&Path, &str) -> Result<Series, String>,
) -> Result<Graph, String> {
    s.validate()?;
    let range = entry
        .map(|e| (e.offset, e.duration, e.fade_in, e.fade_out))
        .unwrap_or((0., t.audible(), 0., 0.));
    let (offset, duration, fade_in, fade_out) = range;
    if [offset, duration, fade_in, fade_out]
        .iter()
        .any(|v| !v.is_finite() || *v < 0.)
        || duration <= 0.
        || duration > 86400.
        || offset + duration > t.audible() + 0.001
        || fade_in > duration
        || fade_out > duration
    {
        return Err("Invalid playback range (maximum 24 hours)".into());
    }
    let source = root.join("sources").join(&t.id);
    let meta = fs::metadata(&source).map_err(|e| e.to_string())?;
    let filters = processing_filter(t, s)?;
    let trim = format!(
        "atrim=start={}:duration={duration},asetpts=PTS-STARTPTS",
        t.trim_start + offset
    );
    let raw_filter = format!("{STEREO},{trim}");
    let mut output_filter = format!("{filters},{trim}");
    if fade_in > 0. {
        output_filter.push_str(&format!(",afade=t=in:st=0:d={fade_in}"));
    }
    if fade_out > 0. {
        output_filter.push_str(&format!(
            ",afade=t=out:st={}:d={fade_out}",
            (duration - fade_out).max(0.)
        ));
    }
    output_filter.push_str(&format!(
        ",aresample={},aformat=sample_fmts=s16:channel_layouts=stereo",
        s.output_sample_rate()
    ));
    let source_identity = (&t.id, meta.len(), meta.modified().ok());
    let original_path = cache::path(
        root,
        "source",
        &("track-series-v1", &source_identity, &raw_filter),
    )?;
    let processed_path = cache::path(
        root,
        "processed",
        &("track-series-v1", &source_identity, &output_filter),
    )?;
    let folder = root.join("loudness-graphs");
    let pair = pair_path(&folder, t, s, range, &meta)?;
    if let Some(data) = cached_graph(&pair, &legacy_path(&folder, t, s, entry, &meta)?) {
        cache::seed(&original_path, &data.original);
        cache::seed(&processed_path, &data.processed);
        return Ok(data);
    }
    let original = cache::resolve(&original_path, || {
        if let Some(original) = existing_original(&folder, t, s, range, &meta) {
            Ok(original)
        } else {
            measure(&source, &raw_filter)
        }
    })?;
    let processed = cache::resolve(&processed_path, || measure(&source, &output_filter))?;
    let data = Graph {
        markers: vec![],
        duration,
        original,
        processed,
    };
    cache::store(&pair, &data)?;
    Ok(data)
}

struct SideIdentity {
    pair: PathBuf,
    legacy: PathBuf,
    original: serde_json::Value,
    filters: Vec<String>,
}
fn side_identity(
    root: &Path,
    tracks: &[Track],
    entries: &[Entry],
    s: &Settings,
) -> Result<SideIdentity, String> {
    let ranges = entries
        .iter()
        .map(|e| (&e.track_id, e.offset, e.duration, e.fade_in, e.fade_out))
        .collect::<Vec<_>>();
    let mut hash = Sha256::new();
    hash.update(
        serde_json::to_vec(&(
            "side-v2",
            &ranges,
            s.side_seconds,
            s.gap,
            s.lead_in,
            s.output_sample_rate(),
        ))
        .map_err(|e| e.to_string())?,
    );
    let mut legacy = Sha256::new();
    legacy.update(serde_json::to_vec(&("side-v1", entries, s)).map_err(|e| e.to_string())?);
    let mut sources = vec![];
    let mut filters = vec![];
    for entry in entries {
        let track = tracks
            .iter()
            .find(|t| t.id == entry.track_id)
            .ok_or("曲が見つかりません")?;
        let meta = fs::metadata(root.join("sources").join(&track.id)).map_err(|e| e.to_string())?;
        let filter = processing_filter(track, s)?;
        hash.update(
            serde_json::to_vec(&(
                &track.id,
                track.trim_start,
                &filter,
                meta.len(),
                meta.modified().ok(),
            ))
            .map_err(|e| e.to_string())?,
        );
        legacy.update(
            serde_json::to_vec(&(track, meta.len(), meta.modified().ok()))
                .map_err(|e| e.to_string())?,
        );
        sources.push(serde_json::json!([
            track.id,
            track.trim_start,
            meta.len(),
            meta.modified().ok()
        ]));
        filters.push(filter);
    }
    let original = serde_json::json!([
        "side-series-v1",
        ranges,
        s.side_seconds,
        s.gap,
        s.lead_in,
        s.output_sample_rate(),
        sources
    ]);
    let folder = root.join("loudness-graphs");
    Ok(SideIdentity {
        pair: folder.join(format!("side-{:x}.json", hash.finalize())),
        legacy: folder.join(format!("side-{:x}.json", legacy.finalize())),
        original,
        filters,
    })
}

/// Aggregate LUFS must be measured across real gaps and track boundaries.
/// Original and processed sides are nevertheless cached independently.
pub fn side_graph(
    root: &Path,
    tracks: &[Track],
    entries: &[Entry],
    s: &Settings,
) -> Result<Graph, String> {
    let folder = root.join("loudness-graphs");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let temp = folder.join(format!("{}.wav", uuid::Uuid::new_v4()));
    let result = side_graph_with_measure(root, tracks, entries, s, &mut |selected, settings| {
        super::render::render(root, selected, entries, settings, &temp)?;
        measure(&temp, "anull")
    });
    let _ = fs::remove_file(temp);
    result
}
fn side_graph_with_measure(
    root: &Path,
    tracks: &[Track],
    entries: &[Entry],
    s: &Settings,
    measure: &mut impl FnMut(&[Track], &Settings) -> Result<Series, String>,
) -> Result<Graph, String> {
    crate::planner::validate_entries(&[entries.to_vec(), vec![]], tracks, s)?;
    let identity = side_identity(root, tracks, entries, s)?;
    let original_path = cache::path(root, "side-source", &identity.original)?;
    let processed_path = cache::path(
        root,
        "side-processed",
        &(&identity.original, &identity.filters),
    )?;
    let mut position = s.lead_in;
    let mut markers = vec![];
    let mut used = vec![];
    for (i, entry) in entries.iter().enumerate() {
        let track = tracks
            .iter()
            .find(|t| t.id == entry.track_id)
            .ok_or("曲が見つかりません")?;
        if i > 0 {
            position += s.gap;
        }
        markers.push(Marker {
            time: position,
            end: position + entry.duration,
            title: track.title.clone(),
        });
        position += entry.duration;
        if !used.iter().any(|old: &Track| old.id == track.id) {
            used.push(track.clone());
        }
    }
    if let Some(mut data) = cached_graph(&identity.pair, &identity.legacy) {
        cache::seed(&original_path, &data.original);
        cache::seed(&processed_path, &data.processed);
        data.markers = markers;
        return Ok(data);
    }
    let raw = Settings {
        normalize: false,
        remove_dc: false,
        ..s.clone()
    };
    let mut originals = used.clone();
    for track in &mut originals {
        track.manual_gain_db = 0.;
        track.pseudo_stereo = 0;
    }
    let original = cache::resolve(&original_path, || {
        let old = side_identity(root, tracks, entries, &raw)?;
        if let Some(data) = cached_graph(&old.pair, &old.legacy) {
            Ok(data.original)
        } else {
            measure(&originals, &raw)
        }
    })?;
    let zero_filter = format!("{STEREO},volume=0dB");
    let unchanged = identity.filters.iter().all(|filter| filter == &zero_filter);
    let processed = cache::resolve(&processed_path, || {
        if unchanged {
            Ok(original.clone())
        } else {
            measure(&used, s)
        }
    })?;
    let data = Graph {
        duration: s.side_seconds,
        markers,
        original,
        processed,
    };
    cache::store(&identity.pair, &data)?;
    Ok(data)
}

#[cfg(test)]
mod reuse_tests {
    use super::*;
    use std::cell::Cell;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("cassette-series-test-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(path.join("sources")).unwrap();
            fs::write(path.join("sources/source"), b"synthetic metadata fixture").unwrap();
            Self(path)
        }
        fn clear_series(&self) {
            for entry in fs::read_dir(self.0.join("loudness-series")).unwrap() {
                fs::remove_file(entry.unwrap().path()).unwrap();
            }
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            assert_eq!(self.0.parent(), Some(std::env::temp_dir().as_path()));
            assert!(
                self.0
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .starts_with("cassette-series-test-")
            );
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn track() -> Track {
        serde_json::from_value(serde_json::json!({
            "id":"source", "title":"Original title", "artist":"", "filename":"test.wav",
            "source":"file", "duration":12., "trim_start":0., "trim_end":12.,
            "loudness":{"integrated_lufs":-24., "true_peak_db":-12.}
        }))
        .unwrap()
    }
    fn measured(calls: &Cell<usize>) -> Series {
        calls.set(calls.get() + 1);
        Series {
            points: vec![Point {
                time: 1.,
                momentary: Some(-24.),
                short_term: None,
            }],
            integrated_lufs: Some(-(calls.get() as f64)),
            true_peak_db: Some(-12.),
        }
    }
    #[test]
    fn analysis_saves_graph_series_using_its_own_ebu_summary() {
        let f = Fixture::new();
        let t = track();
        let log = "[ebur128] t: 0.399 M: -28.0 S: -120.0\n[ebur128] t: 3.099 M: -24.0 S: -25.0\n{\"input_i\":\"-25.79\"}\n    I: -26.4 LUFS\n    Peak: -23.0 dBFS\n";
        save_source_series(&f.0.join("sources/source"), &t, log).unwrap();
        let calls = Cell::new(0);
        let graph = graph_with_measure(
            &f.0,
            &t,
            &Settings::default(),
            None,
            &mut |_: &Path, _: &str| Ok(measured(&calls)),
        )
        .unwrap();
        assert_eq!(
            calls.get(),
            1,
            "Only the processed series still needs measurement"
        );
        assert_eq!(graph.original.integrated_lufs, Some(-26.4));
        assert_eq!(graph.original.true_peak_db, Some(-23.));
        assert_eq!(graph.original.points.len(), 2);
        assert_eq!(graph.original.points[1].short_term, Some(-25.));
    }
    #[test]
    fn track_reuses_original_for_gain_and_fades_but_not_changed_range() {
        let f = Fixture::new();
        let mut t = track();
        let calls = Cell::new(0);
        let mut measure = |_: &Path, _: &str| Ok(measured(&calls));
        let mut s = Settings::default();
        let first = graph_with_measure(&f.0, &t, &s, None, &mut measure).unwrap();
        assert_eq!(calls.get(), 2);
        s.normalize = true;
        let changed = graph_with_measure(&f.0, &t, &s, None, &mut measure).unwrap();
        assert_eq!(calls.get(), 3);
        assert!(first.original == changed.original);
        let mut e = Entry {
            track_id: t.id.clone(),
            offset: 0.,
            duration: 12.,
            fade_in: 0.,
            fade_out: 1.,
            continuation: false,
        };
        graph_with_measure(&f.0, &t, &s, Some(&e), &mut measure).unwrap();
        assert_eq!(calls.get(), 4);
        t.title = "Renamed".into();
        t.mono = Some(true);
        graph_with_measure(&f.0, &t, &s, Some(&e), &mut measure).unwrap();
        assert_eq!(calls.get(), 4);
        e.offset = 1.;
        e.duration = 11.;
        graph_with_measure(&f.0, &t, &s, Some(&e), &mut measure).unwrap();
        assert_eq!(calls.get(), 6);
    }
    #[test]
    fn previous_pair_cache_supplies_original_after_settings_change() {
        let f = Fixture::new();
        let t = track();
        let calls = Cell::new(0);
        let mut measure = |_: &Path, _: &str| Ok(measured(&calls));
        let mut s = Settings::default();
        let first = graph_with_measure(&f.0, &t, &s, None, &mut measure).unwrap();
        f.clear_series();
        s.normalize = true;
        let changed = graph_with_measure(&f.0, &t, &s, None, &mut measure).unwrap();
        assert_eq!(calls.get(), 3);
        assert!(first.original == changed.original);
    }
    #[test]
    fn side_reuses_raw_audio_and_refreshes_titles_without_measurement() {
        let f = Fixture::new();
        let mut tracks = vec![track()];
        let calls = Cell::new(0);
        let mut measure = |_: &[Track], _: &Settings| Ok(measured(&calls));
        let mut s = Settings {
            side_seconds: 30.,
            ..Settings::default()
        };
        let e = Entry {
            track_id: "source".into(),
            offset: 0.,
            duration: 12.,
            fade_in: 0.,
            fade_out: 0.,
            continuation: false,
        };
        let entries = vec![e];
        let first = side_graph_with_measure(&f.0, &tracks, &entries, &s, &mut measure).unwrap();
        assert_eq!(calls.get(), 1);
        assert!(first.original == first.processed);
        s.normalize = true;
        let changed = side_graph_with_measure(&f.0, &tracks, &entries, &s, &mut measure).unwrap();
        assert_eq!(calls.get(), 2);
        assert!(first.original == changed.original);
        tracks[0].title = "New title".into();
        let renamed = side_graph_with_measure(&f.0, &tracks, &entries, &s, &mut measure).unwrap();
        assert_eq!(calls.get(), 2);
        assert_eq!(renamed.markers[0].title, "New title");
        s.lead_in = 1.;
        side_graph_with_measure(&f.0, &tracks, &entries, &s, &mut measure).unwrap();
        assert_eq!(calls.get(), 4);
    }
}
