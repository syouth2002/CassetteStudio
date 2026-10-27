use super::analysis::{parse_dc, parse_loudness, silence_bounds};
use super::*;
use crate::model::*;
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
        dc: None,
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
fn dc_parsing_gain_and_cache_are_independent_of_raw_measurements() {
    assert_eq!(
        parse_dc("DC offset: 0.12\nDC offset: -0.07").unwrap(),
        [0.12, -0.07]
    );
    for invalid in [
        "",
        "DC offset: 0.1",
        "DC offset: NaN\nDC offset: 0",
        "DC offset: 0\nDC offset: 0\nDC offset: 0",
    ] {
        assert!(parse_dc(invalid).is_err());
    }
    let mut t = test_track();
    let s = Settings {
        normalize: true,
        remove_dc: true,
        ..Settings::default()
    };
    assert!(gain(&t, &s).is_err());
    t.dc = Some(DcAnalysis {
        offset: [0.12, -0.07],
        corrected_loudness: Loudness {
            integrated_lufs: Some(-30.0),
            true_peak_db: Some(-20.0),
        },
    });
    assert_eq!(gain(&t, &s).unwrap().gain_db, 12.0);
    assert_eq!(
        gain(
            &t,
            &Settings {
                remove_dc: false,
                ..s.clone()
            }
        )
        .unwrap()
        .gain_db,
        6.0
    );
    let corrected = preview_name(&t, &s);
    assert_ne!(
        corrected,
        preview_name(
            &t,
            &Settings {
                remove_dc: false,
                ..s.clone()
            }
        )
    );
    t.dc.as_mut().unwrap().offset[0] = 0.2;
    assert_ne!(corrected, preview_name(&t, &s));
    assert!(!serde_json::from_str::<Settings>("{}").unwrap().remove_dc);
    let mut legacy = serde_json::to_value(&t).unwrap();
    legacy.as_object_mut().unwrap().remove("dc");
    assert!(
        serde_json::from_value::<Track>(legacy)
            .unwrap()
            .dc
            .is_none()
    );
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
