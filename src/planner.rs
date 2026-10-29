use crate::model::*;

fn whole(t: &Track) -> Entry {
    Entry {
        track_id: t.id.clone(),
        offset: 0.0,
        duration: t.audible(),
        fade_in: 0.0,
        fade_out: 0.0,
        continuation: false,
    }
}

// Bounded subset sum at 0.1 s resolution. Round costs up so an exact
// sample duration can never overflow a side due to quantization.
fn select(pool: &[&Track], capacity: f64, gap: f64, has_first: bool) -> Vec<usize> {
    let budget = ((capacity + if has_first { 0.0 } else { gap }) * 10.0)
        .floor()
        .max(0.0) as usize;
    let mut paths: Vec<Option<(usize, usize)>> = vec![None; budget + 1];
    paths[0] = Some((usize::MAX, 0));
    // Immutable linked nodes avoid a later DP update changing an earlier path.
    let mut nodes: Vec<(usize, usize)> = vec![(usize::MAX, 0)];
    for (i, t) in pool.iter().enumerate() {
        let cost = ((t.audible() + gap) * 10.0).ceil() as usize;
        if cost == 0 || cost > budget {
            continue;
        }
        for n in (cost..=budget).rev() {
            if paths[n].is_none()
                && let Some((_, previous)) = paths[n - cost]
            {
                nodes.push((i, previous));
                paths[n] = Some((i, nodes.len() - 1));
            }
        }
    }
    let mut result = vec![];
    let mut node = paths
        .iter()
        .rev()
        .find_map(|p| p.map(|(_, node)| node))
        .unwrap_or(0);
    while node != 0 {
        let (i, prev) = nodes[node];
        result.push(i);
        node = prev;
    }
    result.sort_unstable();
    result
}

pub fn plan(
    tracks: &[Track],
    ids: &[String],
    s: &Settings,
    optimize: bool,
) -> Result<[Vec<Entry>; 2], String> {
    s.validate()?;
    let mut pool: Vec<&Track> = Vec::new();
    for id in ids {
        let t = tracks
            .iter()
            .find(|t| &t.id == id)
            .ok_or("見つからない曲があります")?;
        if t.audible() <= 0.0 {
            continue;
        }
        pool.push(t);
    }
    if pool.len() > 2000 {
        return Err("一度に配置できる曲は2000曲までです".into());
    }
    let mut sides: [Vec<Entry>; 2] = [vec![], vec![]];
    for side in 0..2 {
        let remaining = s.content_seconds() - side_duration(&sides[side], s.gap);
        let chosen = if optimize {
            select(&pool, remaining, s.gap, !sides[side].is_empty())
        } else {
            let mut used = 0.0;
            let mut indices = vec![];
            for (i, t) in pool.iter().enumerate() {
                let cost = t.audible()
                    + if !indices.is_empty() || !sides[side].is_empty() {
                        s.gap
                    } else {
                        0.0
                    };
                if used + cost > remaining + 1e-8 {
                    break;
                }
                used += cost;
                indices.push(i);
            }
            indices
        };
        for &i in &chosen {
            sides[side].push(whole(pool[i]));
        }
        for &i in chosen.iter().rev() {
            pool.remove(i);
        }
        let room = s.content_seconds() - side_duration(&sides[side], s.gap);
        let available = room - if sides[side].is_empty() { 0.0 } else { s.gap };
        if room + 1e-8 >= s.fill_threshold && available > 0.0 && !pool.is_empty() {
            let t = pool.remove(0);
            let mut e = whole(t);
            if e.duration > available + 1e-8 {
                e.duration = available;
                e.fade_out = s.fade.min(available);
                if side == 0 {
                    let mut next = whole(t);
                    next.continuation = true;
                    if s.continuation == "resume" {
                        next.offset = available;
                        next.duration = t.audible() - next.offset;
                        next.fade_in = s.fade.min(next.duration);
                    }
                    if next.duration > s.content_seconds() {
                        next.duration = s.content_seconds();
                        next.fade_out = s.fade.min(next.duration);
                        next.fade_in = next.fade_in.min(next.duration);
                    }
                    sides[1].push(next);
                }
            }
            sides[side].push(e);
        }
    }
    Ok(sides)
}

#[derive(Debug, serde::Serialize)]
pub struct BatchPlan {
    pub collections: Vec<Collection>,
    pub remaining: usize,
    pub excluded: usize,
}

/// Reserve each recording once across all destination tapes. A/B continuation
/// is produced by plan() inside one tape and does not consume another recording.
pub fn fill_cassettes(
    lib: &Library,
    ids: &[String],
    targets: &[String],
) -> Result<BatchPlan, String> {
    use std::collections::HashSet;
    if targets.is_empty() || targets.len() > 100 {
        return Err("空のカセットを1〜100本選択してください".into());
    }
    let mut seen_targets = HashSet::new();
    let mut destinations = Vec::new();
    for id in targets {
        if !seen_targets.insert(id) {
            return Err("同じカセットが重複しています".into());
        }
        let c = lib
            .collections
            .iter()
            .find(|c| &c.id == id)
            .ok_or("カセットが見つかりません")?;
        if c.kind != "cassette" || !c.sides.iter().all(Vec::is_empty) {
            return Err("配置先には空のカセットだけを選択してください".into());
        }
        c.settings.validate()?;
        destinations.push(c);
    }
    let mut used: HashSet<String> = lib
        .collections
        .iter()
        .filter(|c| c.kind == "cassette")
        .flat_map(|c| c.sides.iter().flatten().map(|e| e.track_id.clone()))
        .collect();
    let mut seen = HashSet::new();
    let mut pool = Vec::new();
    let mut excluded = 0;
    for id in ids {
        if !seen.insert(id) {
            continue;
        }
        let t = lib
            .tracks
            .iter()
            .find(|t| &t.id == id)
            .ok_or("曲が見つかりません")?;
        if used.contains(id) {
            excluded += 1;
        } else if t.audible() > 0.0 {
            pool.push(id.clone());
        }
    }
    if pool.is_empty() {
        return Err("未収録の曲がありません。収録済みの曲と無音の曲は除外します".into());
    }
    let mut collections = Vec::new();
    for target in destinations {
        if pool.is_empty() {
            break;
        }
        let sides = plan(&lib.tracks, &pool, &target.settings, true)?;
        validate_entries(&sides, &lib.tracks, &target.settings)?;
        if sides.iter().all(Vec::is_empty) {
            continue;
        }
        let mut c = target.clone();
        c.tracks = sides
            .iter()
            .flatten()
            .map(|e| e.track_id.clone())
            .filter(|id| used.insert(id.clone()))
            .collect();
        c.sides = sides;
        pool.retain(|id| !used.contains(id));
        collections.push(c);
    }
    if collections.is_empty() {
        return Err("指定した時間・余白の設定では曲を配置できませんでした".into());
    }
    Ok(BatchPlan {
        collections,
        remaining: pool.len(),
        excluded,
    })
}

pub fn validate_entries(
    sides: &[Vec<Entry>; 2],
    tracks: &[Track],
    s: &Settings,
) -> Result<(), String> {
    s.validate()?;
    for side in sides {
        if side_duration(side, s.gap) > s.content_seconds() + 0.001 {
            return Err("片面の時間を超えています。曲を移動するか、再配置してください".into());
        }
        for e in side {
            let t = tracks
                .iter()
                .find(|t| t.id == e.track_id)
                .ok_or("曲が見つかりません")?;
            if [e.offset, e.duration, e.fade_in, e.fade_out]
                .iter()
                .any(|v| !v.is_finite() || *v < 0.0)
                || e.duration <= 0.0
                || e.offset + e.duration > t.audible() + 0.01
                || e.fade_in > e.duration
                || e.fade_out > e.duration
            {
                return Err("曲の再生範囲が不正です".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn track(id: &str, len: f64) -> Track {
        Track {
            id: id.into(),
            title: id.into(),
            artist: String::new(),
            filename: String::new(),
            source: String::new(),
            content_hash: None,
            duration: len + 10.0,
            trim_start: 4.0,
            trim_end: len + 4.0,
            manual_gain_db: 0.0,
            loudness: None,
            dc: None,
        }
    }
    #[test]
    fn silence_and_gap_count() {
        let t = vec![track("a", 40.0), track("b", 58.0)];
        let s = Settings {
            side_seconds: 100.0,
            gap: 2.0,
            ..Settings::default()
        };
        let p = plan(&t, &["a".into(), "b".into()], &s, true).unwrap();
        assert_eq!(side_duration(&p[0], s.gap), 100.0);
        assert_eq!(p[0][0].duration, 40.0);
    }

    #[test]
    fn leading_silence_is_reserved_on_both_sides_and_continuations() {
        let tracks = vec![track("a", 12.0)];
        let s = Settings {
            side_seconds: 10.0,
            lead_in: 2.5,
            fill_threshold: 0.0,
            continuation: "resume".into(),
            ..Settings::default()
        };
        for optimize in [false, true] {
            let sides = plan(&tracks, &["a".into()], &s, optimize).unwrap();
            assert_eq!(sides[0][0].duration, 7.5);
            assert_eq!(sides[1][0].offset, 7.5);
            assert_eq!(sides[1][0].duration, 4.5);
            validate_entries(&sides, &tracks, &s).unwrap();
            let mut overflow = sides;
            overflow[0][0].duration = 8.0;
            assert!(validate_entries(&overflow, &tracks, &s).is_err());
        }
    }
    fn blank(id: &str, seconds: f64) -> Collection {
        Collection {
            id: id.into(),
            name: id.into(),
            kind: "cassette".into(),
            tracks: vec![],
            sides: [vec![], vec![]],
            settings: Settings {
                side_seconds: seconds,
                gap: 0.0,
                fill_threshold: 0.0,
                ..Settings::default()
            },
        }
    }
    #[test]
    fn batch_uses_saved_lengths_and_excludes_existing_and_repeated_ids() {
        let tracks = vec![
            track("used", 30.0),
            track("a", 60.0),
            track("b", 80.0),
            track("c", 40.0),
            track("d", 90.0),
            track("e", 120.0),
        ];
        let mut existing = blank("existing", 100.0);
        existing.sides[0].push(whole(&tracks[0]));
        let lib = Library {
            tracks,
            collections: vec![existing.clone(), blank("short", 50.0), blank("long", 100.0)],
        };
        let ids = ["used", "a", "a", "b", "c", "d", "e"].map(String::from);
        let result = fill_cassettes(&lib, &ids, &["short".into(), "long".into()]).unwrap();
        assert_eq!(result.collections.len(), 2);
        assert_eq!(result.excluded, 1);
        assert_eq!(lib.collections[0], existing);
        let mut used = std::collections::HashSet::new();
        for c in &result.collections {
            validate_entries(&c.sides, &lib.tracks, &c.settings).unwrap();
            for id in &c.tracks {
                assert_ne!(id, "used");
                assert!(used.insert(id.clone()));
            }
        }
        assert_eq!(result.collections[0].settings.side_seconds, 50.0);
        assert_eq!(result.collections[1].settings.side_seconds, 100.0);
        assert_eq!(result.remaining + used.len(), 5);
        assert!(
            result
                .collections
                .iter()
                .flat_map(|c| &c.sides[1])
                .any(|e| e.continuation)
        );
    }
    #[test]
    fn batch_leaves_unused_tapes_empty_and_rejects_filled_destinations() {
        let lib = Library {
            tracks: vec![track("a", 20.0)],
            collections: vec![blank("one", 50.0), blank("two", 50.0)],
        };
        let r = fill_cassettes(
            &lib,
            &["a".into(), "a".into()],
            &["one".into(), "two".into()],
        )
        .unwrap();
        assert_eq!(r.collections.len(), 1);
        assert_eq!(r.remaining, 0);
        assert!(lib.collections[1].sides.iter().all(Vec::is_empty));
        let mut filled = lib.clone();
        filled.collections[0] = r.collections[0].clone();
        assert!(fill_cassettes(&filled, &["a".into()], &["one".into()]).is_err());
        assert!(fill_cassettes(&filled, &["a".into()], &["two".into()]).is_err());
        assert!(fill_cassettes(&lib, &["a".into()], &["one".into(), "one".into()]).is_err());
    }
    #[test]
    fn restart_and_resume() {
        let t = vec![track("a", 60.0), track("b", 80.0)];
        let mut s = Settings {
            side_seconds: 100.0,
            gap: 2.0,
            fill_threshold: 20.0,
            ..Settings::default()
        };
        let ids = vec!["a".into(), "b".into()];
        let p = plan(&t, &ids, &s, false).unwrap();
        assert_eq!(p[0][1].duration, 38.0);
        assert_eq!(p[1][0].offset, 0.0);
        assert_eq!(p[1][0].duration, 80.0);
        s.continuation = "resume".into();
        let p = plan(&t, &ids, &s, false).unwrap();
        assert_eq!(p[1][0].offset, 38.0);
        assert_eq!(p[1][0].duration, 42.0);
        assert_eq!(p[1][0].fade_in, 5.0);
        validate_entries(&p, &t, &s).unwrap();
    }
    #[test]
    fn long_track_and_threshold() {
        let t = vec![track("a", 300.0)];
        let s = Settings {
            side_seconds: 100.0,
            ..Settings::default()
        };
        let p = plan(&t, &["a".into()], &s, true).unwrap();
        validate_entries(&p, &t, &s).unwrap();
        assert_eq!(p[1][0].fade_out, 5.0);
        let s = Settings {
            fill_threshold: 101.0,
            ..s
        };
        assert!(plan(&t, &["a".into()], &s, true).unwrap()[0].is_empty());
    }
    #[test]
    fn subset_has_no_repeated_tracks_and_never_overflows() {
        for seed in 0..20 {
            let t: Vec<_> = (0..30)
                .map(|i| track(&i.to_string(), ((i * 73 + seed * 31) % 400) as f64 + 0.03))
                .collect();
            let s = Settings {
                side_seconds: 900.0,
                ..Settings::default()
            };
            let ids = t.iter().map(|t| t.id.clone()).collect::<Vec<_>>();
            let p = plan(&t, &ids, &s, true).unwrap();
            validate_entries(&p, &t, &s).unwrap();
            for side in &p {
                let mut used = std::collections::HashSet::new();
                for e in side {
                    assert!(used.insert(&e.track_id));
                }
            }
        }
    }
}
