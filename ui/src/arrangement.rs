//! Manual recording order, side capacity and A-to-B continuation.
use crate::model::*;

pub fn whole(track: &Track) -> Entry {
    Entry {
        track_id: track.id.clone(),
        offset: 0.,
        duration: track.audible(),
        fade_in: 0.,
        fade_out: 0.,
        continuation: false,
    }
}

pub fn fit(entry: &mut Entry, available: f64, source: f64, fade: f64) -> bool {
    let remaining = (source - entry.offset).max(0.);
    if available < 0.1 || remaining <= 0. {
        return false;
    }
    let cut = remaining > available + 1e-6;
    entry.duration = if cut {
        (available * 10. + 1e-8).floor() / 10.
    } else {
        remaining
    };
    entry.fade_out = if cut {
        fade.min((entry.duration * 2.).floor() / 2.)
    } else {
        0.
    };
    entry.fade_in = entry.fade_in.min(entry.duration);
    true
}

fn available(collection: &Collection, side: usize) -> f64 {
    collection.settings.content_seconds()
        - side_duration(&collection.sides[side], collection.settings.gap)
        - if collection.sides[side].is_empty() {
            0.
        } else {
            collection.settings.gap
        }
}

/// Append in selection order, advancing to B once A fills or fades out.
/// Return the number of selected sources placed, excluding continuation copies.
pub fn append(
    collection: &mut Collection,
    tracks: &[Track],
    ids: &[String],
    start_side: usize,
) -> usize {
    let mut side = start_side.min(1);
    let mut added = 0;
    for id in ids {
        let Some(track) = tracks.iter().find(|t| &t.id == id) else {
            continue;
        };
        if collection.kind == "playlist" {
            collection.tracks.push(id.clone());
            added += 1;
            continue;
        }
        if track.audible() <= 0. {
            continue;
        }
        let mut entry = whole(track);
        loop {
            if !fit(
                &mut entry,
                available(collection, side),
                track.audible(),
                collection.settings.fade,
            ) {
                if side == 0 {
                    side = 1;
                    continue;
                }
                break;
            }
            let cut = entry.duration < track.audible() - 1e-6;
            collection.sides[side].push(entry.clone());
            added += 1;
            if side == 0 && cut {
                side = 1;
                // Preserve existing B recordings. A new tape starts B with the
                // configured restart or fade-in continuation of the split song.
                if collection.sides[1].is_empty() {
                    let mut next = whole(track);
                    next.continuation = true;
                    if collection.settings.continuation == "resume" {
                        next.offset = entry.duration;
                        next.fade_in = collection.settings.fade;
                    }
                    if fit(
                        &mut next,
                        available(collection, 1),
                        track.audible(),
                        collection.settings.fade,
                    ) {
                        collection.sides[1].push(next);
                    }
                }
            }
            break;
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;
    fn track(id: usize, length: f64) -> Track {
        serde_json::from_value(serde_json::json!({"id":id.to_string(),"title":id.to_string(),"artist":"Test","filename":"test.wav","source":"local","duration":length,"trim_start":0.,"trim_end":length})).unwrap()
    }
    fn tape(seconds: f64) -> Collection {
        Collection {
            id: "tape".into(),
            name: "Tape".into(),
            kind: "cassette".into(),
            tracks: vec![],
            sides: [vec![], vec![]],
            settings: Settings {
                side_seconds: seconds,
                gap: 1.,
                fade: 2.,
                ..Default::default()
            },
        }
    }
    fn ids(tracks: &[Track]) -> Vec<String> {
        tracks.iter().map(|t| t.id.clone()).collect()
    }
    fn within_capacity(c: &Collection) {
        for side in &c.sides {
            assert!(
                c.settings.lead_in + side_duration(side, c.settings.gap)
                    <= c.settings.side_seconds + 1e-6
            );
        }
    }
    #[test]
    fn selection_larger_than_a_is_appended_to_b_in_order() {
        let tracks = (0..6).map(|i| track(i, 8.)).collect::<Vec<_>>();
        let mut c = tape(26.);
        assert_eq!(append(&mut c, &tracks, &ids(&tracks), 0), 6);
        assert_eq!(
            c.sides[0]
                .iter()
                .map(|e| e.track_id.as_str())
                .collect::<Vec<_>>(),
            ["0", "1", "2"]
        );
        assert_eq!(
            c.sides[1]
                .iter()
                .map(|e| e.track_id.as_str())
                .collect::<Vec<_>>(),
            ["3", "4", "5"]
        );
        within_capacity(&c);
    }
    #[test]
    fn later_songs_follow_the_split_song_on_b_in_both_continuation_modes() {
        let tracks = vec![track(0, 6.), track(1, 6.), track(2, 2.)];
        for mode in ["restart", "resume"] {
            let mut c = tape(10.);
            c.settings.continuation = mode.into();
            assert_eq!(append(&mut c, &tracks, &ids(&tracks), 0), 3);
            assert_eq!(c.sides[0][1].duration, 3.);
            assert_eq!(c.sides[0][1].fade_out, 2.);
            assert_eq!(
                c.sides[1]
                    .iter()
                    .map(|e| e.track_id.as_str())
                    .collect::<Vec<_>>(),
                ["1", "2"]
            );
            assert!(c.sides[1][0].continuation);
            assert_eq!(c.sides[1][0].offset, if mode == "resume" { 3. } else { 0. });
            assert_eq!(
                c.sides[1][0].fade_in,
                if mode == "resume" { 2. } else { 0. }
            );
            within_capacity(&c);
        }
    }
    #[test]
    fn appending_to_a_full_a_preserves_existing_b_recordings_and_gaps() {
        let tracks = vec![track(0, 10.), track(1, 3.), track(2, 4.)];
        let mut c = tape(10.);
        c.sides = [vec![whole(&tracks[0])], vec![whole(&tracks[1])]];
        assert_eq!(append(&mut c, &tracks, &["2".into()], 0), 1);
        assert_eq!(c.sides[1][0], whole(&tracks[1]));
        assert_eq!(c.sides[1][1], whole(&tracks[2]));
        within_capacity(&c);
    }
    #[test]
    fn lead_in_is_reserved_on_both_sides_and_full_tapes_keep_their_entries() {
        let tracks = (0..4).map(|i| track(i, 4.)).collect::<Vec<_>>();
        let mut c = tape(12.);
        c.settings.lead_in = 2.;
        assert_eq!(append(&mut c, &tracks, &ids(&tracks), 0), 4);
        assert_eq!(c.sides[0].len(), 2);
        assert_eq!(c.sides[1].len(), 2);
        within_capacity(&c);
        let before = c.clone();
        assert_eq!(append(&mut c, &tracks, &ids(&tracks), 0), 0);
        assert_eq!(c, before);
    }
    #[test]
    fn direct_b_additions_do_not_wrap_back_to_a() {
        let tracks = vec![track(0, 10.), track(1, 3.)];
        let mut c = tape(10.);
        assert_eq!(append(&mut c, &tracks, &ids(&tracks), 1), 1);
        assert!(c.sides[0].is_empty());
        assert_eq!(c.sides[1].len(), 1);
    }

    #[test]
    fn silent_sources_do_not_force_a_side_change() {
        let tracks = vec![track(0, 0.), track(1, 4.)];
        let mut c = tape(10.);
        assert_eq!(append(&mut c, &tracks, &ids(&tracks), 0), 1);
        assert_eq!(c.sides[0][0].track_id, "1");
        assert!(c.sides[1].is_empty());
    }
}
