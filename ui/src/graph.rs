use crate::{model::*, settings::tr};
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use serde_json::Value;
#[derive(Default)]
pub struct Graph {
    pub value: Value,
    pub generation: u64,
    pub collection: Option<String>,
    pub side: usize,
    pub show: bool,
    pub loading: bool,
    pub error: Option<String>,
    pub short_term: bool,
    pub original: bool,
    pub processed: bool,
    pub title: String,
    pub target_lufs: f64,
    pub normalization_preview: bool,
    cache: std::collections::VecDeque<(String, Value)>,
    pending: std::collections::HashMap<u64, String>,
    current_key: Option<String>,
}

fn source_key(track: &Track) -> Value {
    serde_json::json!([
        track.id,
        track.duration,
        track.trim_start,
        track.trim_end,
        track.manual_gain_db,
        track.pseudo_stereo
    ])
}
fn processed_source_key(track: &Track, settings: &Settings) -> Value {
    serde_json::json!([
        source_key(track),
        if settings.normalize
            || settings.remove_dc
            || track.manual_gain_db != 0.
            || track.pseudo_stereo != 0
        {
            serde_json::json!([
                track.loudness,
                if settings.remove_dc {
                    track.dc.as_ref()
                } else {
                    None
                },
                if track.pseudo_stereo != 0 {
                    track.pseudo_analysis.as_ref()
                } else {
                    None
                }
            ])
        } else {
            Value::Null
        }
    ])
}
fn processing_key(settings: &Settings) -> Value {
    serde_json::json!([
        settings.normalize,
        if settings.normalize {
            Some(settings.target_lufs)
        } else {
            None
        },
        settings.remove_dc,
        settings.peak_db,
        settings.output_sample_rate()
    ])
}
pub fn track_key(track: &Track, settings: &Settings, entry: Option<&Entry>) -> String {
    serde_json::json!([
        "track",
        processed_source_key(track, settings),
        processing_key(settings),
        entry
            .map(|e| (e.offset, e.duration, e.fade_in, e.fade_out))
            .unwrap_or((0., track.audible(), 0., 0.))
    ])
    .to_string()
}
pub fn side_key(collection: &Collection, tracks: &[Track], side: usize) -> String {
    let entries = collection.sides[side]
        .iter()
        .map(|e| {
            serde_json::json!([
                e.track_id,
                e.offset,
                e.duration,
                e.fade_in,
                e.fade_out,
                tracks
                    .iter()
                    .find(|t| t.id == e.track_id)
                    .map(|t| processed_source_key(t, &collection.settings))
            ])
        })
        .collect::<Vec<_>>();
    serde_json::json!([
        "side",
        entries,
        processing_key(&collection.settings),
        collection.settings.side_seconds,
        collection.settings.gap,
        collection.settings.lead_in
    ])
    .to_string()
}

impl Graph {
    pub fn request(&mut self, key: String) -> Option<u64> {
        self.show = true;
        if self.loading && self.current_key.as_ref() == Some(&key) {
            return None;
        }
        self.generation += 1;
        self.current_key = Some(key.clone());
        self.error = None;
        if let Some(index) = self.cache.iter().position(|(saved, _)| saved == &key) {
            let cached = self.cache.remove(index).unwrap();
            self.value = cached.1.clone();
            self.cache.push_back(cached);
            self.loading = false;
            self.original = true;
            self.processed = true;
            return None;
        }
        self.begin();
        self.pending.insert(self.generation, key);
        Some(self.generation)
    }
    pub fn complete(&mut self, generation: u64, value: Value) {
        if let Some(key) = self.pending.remove(&generation) {
            self.cache.retain(|(old, _)| old != &key);
            self.cache.push_back((key, value.clone()));
            while self.cache.len() > 16 {
                self.cache.pop_front();
            }
        }
        if generation == self.generation {
            self.value = value;
            self.loading = false;
            self.error = None;
            self.original = true;
            self.processed = true;
        }
    }
    pub fn begin(&mut self) {
        self.show = true;
        self.loading = true;
        self.error = None;
        self.value = Value::Null;
    }
    pub fn failed(&mut self, generation: u64, error: String) {
        self.pending.remove(&generation);
        if generation == self.generation {
            self.loading = false;
            self.error = Some(error);
        }
    }
    pub fn ui(&mut self, ctx: &egui::Context, ja: bool) -> Option<(String, usize)> {
        let mut request = None;
        let mut open = self.show;
        egui::Window::new(tr(ja, "LUFSグラフ", "LUFS graph"))
            .open(&mut open)
            .default_size([1000., 650.])
            .resizable(true)
            .show(ctx, |ui| {
                if self.loading {
                    ui.set_min_size(Vec2::new(650.,420.));
                    ui.centered_and_justified(|ui| {
                        ui.vertical_centered(|ui| {
                            ui.spinner();
                            ui.heading(tr(ja,"LUFSグラフを読み込み中…","Loading LUFS graph…"));
                            ui.label(tr(ja,"保存済みの結果を確認しています。グラフが未保存の場合のみ解析します。","Checking saved results. Audio is analyzed only when this graph has not been saved."));
                        });
                    });
                    return;
                }
                if let Some(error)=&self.error {
                    ui.colored_label(Color32::DARK_RED,tr(ja,"LUFS解析に失敗しました","LUFS analysis failed"));
                    ui.label(error);
                    return;
                }
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(180.);
                        if let Some(id) = self.collection.clone() {
                            for side in 0..2 {
                                if ui
                                    .selectable_label(
                                        self.side == side,
                                        if side == 0 { "SIDE A" } else { "SIDE B" },
                                    )
                                    .clicked()
                                    && self.side!=side
                                {
                                    self.side = side;
                                    request = Some((id.clone(), side));
                                }
                            }
                        }
                        ui.label(&self.title);
                        if self.normalization_preview {
                            ui.label(tr(ja, "音量統一プレビュー", "Normalization preview"));
                            ui.label(format!("{} {:.1} LUFS", tr(ja, "目標", "Target"), self.target_lufs));
                        }
                        ui.checkbox(
                            &mut self.short_term,
                            tr(ja, "短期（3秒）", "Short-term (3 s)"),
                        );
                        ui.checkbox(&mut self.original, tr(ja, "元の音量（赤点線）", "Original (red dash)"));
                        ui.checkbox(&mut self.processed, if self.normalization_preview { tr(ja, "音量統一後（実線）", "Normalized (solid)") } else { tr(ja, "処理後（実線）", "Processed (solid)") });
                        let g = if self.value["graph"].is_object() {
                            &self.value["graph"]
                        } else {
                            &self.value
                        };
                        if g["original"]["points"] == g["processed"]["points"] {
                            ui.label(tr(ja, "処理前後のグラフは重なっています。", "Original and processed curves overlap."));
                        }
                        for name in ["original", "processed"] {
                            ui.label(if name == "original" {
                                tr(ja, "元の音量", "Original")
                            } else {
                                tr(ja, "処理後", "Processed")
                            });
                            for key in ["integrated_lufs", "true_peak_db"] {
                                if let Some(n) = g[name][key].as_f64() {
                                    let label = if key == "integrated_lufs" {
                                        tr(ja, "統合音量（LUFS）", "Integrated LUFS")
                                    } else {
                                        tr(ja, "ピーク（dBTP）", "True peak (dBTP)")
                                    };
                                    ui.small(format!("{label}: {n:.1}"));
                                }
                            }
                        }
                    });
                    let (r, response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), ui.available_height().max(300.)),
                        egui::Sense::hover(),
                    );
                    let p = ui.painter_at(r);
                    p.rect_filled(r, 4., Color32::from_rgb(22, 29, 33));
                    let g = if self.value["graph"].is_object() {
                        &self.value["graph"]
                    } else {
                        &self.value
                    };
                    let length = g["duration"].as_f64().unwrap_or(1.).max(0.01);
                    let plot = r.shrink2(Vec2::new(40., 22.));
                    let point = |t: f64, v: f64| {
                        Pos2::new(
                            plot.left() + (t / length) as f32 * plot.width(),
                            plot.bottom() - ((v + 60.) / 60.) as f32 * plot.height(),
                        )
                    };
                    for db in [-60, -50, -40, -30, -20, -10, 0] {
                        let y = point(0., db as f64).y;
                        p.line_segment(
                            [Pos2::new(plot.left(), y), Pos2::new(plot.right(), y)],
                            Stroke::new(1.0_f32, Color32::DARK_GRAY),
                        );
                        p.text(
                            Pos2::new(plot.left() - 5., y),
                            egui::Align2::RIGHT_CENTER,
                            db.to_string(),
                            egui::FontId::monospace(12.),
                            Color32::LIGHT_GRAY,
                        );
                    }
                    let target = self.target_lufs;
                    p.line_segment(
                        [point(0., target), point(length, target)],
                        Stroke::new(1.0_f32, Color32::YELLOW),
                    );
                    for (key, show, color) in [
                        ("original", self.original, Color32::from_rgb(244, 90, 90)),
                        ("processed", self.processed, Color32::from_rgb(61, 210, 173)),
                    ] {
                        if !show {
                            continue;
                        }
                        let mut line = Vec::new();
                        for v in g[key]["points"].as_array().into_iter().flatten() {
                            if let (Some(t), Some(db)) = (
                                v["time"].as_f64(),
                                v[if self.short_term {
                                    "short_term"
                                } else {
                                    "momentary"
                                }]
                                .as_f64(),
                            ) {
                                line.push(point(t, db.clamp(-60., 0.)));
                            } else if line.len() > 1 {
                                draw_series(&p,&line,color,key=="original");
                                line.clear();
                            } else {
                                line.clear();
                            }
                        }
                        if line.len() > 1 {
                            draw_series(&p,&line,color,key=="original");
                        }
                    }
                    for m in g["markers"].as_array().into_iter().flatten() {
                        if let Some(t) = m["time"].as_f64() {
                            let x = point(t, 0.).x;
                            p.line_segment(
                                [Pos2::new(x, plot.top()), Pos2::new(x, plot.bottom())],
                                Stroke::new(0.7f32, Color32::from_gray(75)),
                            );
                        }
                    }
                    if let Some(pos) = response.hover_pos() {
                        let t =
                            ((pos.x - plot.left()) / plot.width()).clamp(0., 1.) as f64 * length;
                        response.on_hover_text(format!(
                            "{} / {}",
                            crate::settings::time(t),
                            crate::settings::time(length)
                        ));
                    }
                });
            });
        self.show = open;
        request
    }
}

fn draw_series(painter: &egui::Painter, line: &[Pos2], color: Color32, dashed: bool) {
    if dashed {
        for shape in egui::Shape::dashed_line(line, Stroke::new(1.8_f32, color), 6., 5.) {
            painter.add(shape);
        }
    } else {
        painter.add(egui::Shape::line(
            line.to_vec(),
            Stroke::new(1.8_f32, color),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_new_analysis_clears_old_data_and_ignores_stale_failures() {
        let mut graph = Graph {
            generation: 2,
            value: serde_json::json!({"old":true}),
            ..Default::default()
        };
        graph.begin();
        assert!(graph.show && graph.loading && graph.value.is_null());
        graph.failed(1, "stale".into());
        assert!(graph.loading && graph.error.is_none());
        graph.failed(2, "current".into());
        assert!(!graph.loading);
        assert_eq!(graph.error.as_deref(), Some("current"));
    }

    #[test]
    fn completed_and_pending_graphs_are_reused_after_closing() {
        let mut graph = Graph::default();
        let generation = graph.request("A".into()).unwrap();
        graph.show = false;
        assert_eq!(graph.request("A".into()), None);
        assert_eq!(graph.generation, generation);
        graph.complete(generation, serde_json::json!({"result":"A"}));
        let other = graph.request("B".into()).unwrap();
        graph.complete(other, serde_json::json!({"result":"B"}));
        graph.show = false;
        assert_eq!(graph.request("A".into()), None);
        assert!(!graph.loading);
        assert_eq!(graph.value["result"], "A");
    }

    #[test]
    fn normalized_graph_cache_changes_when_shared_analysis_changes() {
        let mut track: Track = serde_json::from_value(serde_json::json!({"id":"test","title":"Title","artist":"Artist","filename":"file.wav","source":"local","duration":100.,"trim_start":1.,"trim_end":99.})).unwrap();
        let settings = Settings::default().loudness_preview();
        track.loudness = Some(Loudness {
            integrated_lufs: Some(-24.),
            true_peak_db: Some(-8.),
        });
        let before = track_key(&track, &settings, None);
        track.loudness.as_mut().unwrap().integrated_lufs = Some(-22.);
        assert_ne!(track_key(&track, &settings, None), before);
    }

    #[test]
    fn cache_identity_follows_audio_changes_not_metadata() {
        let mut track:Track=serde_json::from_value(serde_json::json!({"id":"test","title":"Title","artist":"Artist","filename":"file.wav","source":"local","duration":100.,"trim_start":1.,"trim_end":99.})).unwrap();
        let settings = Settings::default();
        let original = track_key(&track, &settings, None);
        track.title = "Renamed".into();
        track.artist = "New artist".into();
        track.mono = Some(true);
        track.content_hash = Some("newly measured identity".into());
        track.loudness = Some(Loudness {
            integrated_lufs: Some(-18.),
            true_peak_db: Some(-1.),
        });
        assert_eq!(track_key(&track, &settings, None), original);
        let mut changed = settings.clone();
        changed.silence_db = -40.;
        changed.gap = 5.;
        assert_eq!(track_key(&track, &changed, None), original);
        track.manual_gain_db = 2.;
        assert_ne!(track_key(&track, &settings, None), original);
        let collection = Collection {
            id: "tape".into(),
            name: "Tape".into(),
            kind: "cassette".into(),
            tracks: vec![],
            sides: [
                vec![Entry {
                    track_id: track.id.clone(),
                    offset: 0.,
                    duration: 90.,
                    fade_in: 0.,
                    fade_out: 2.,
                    continuation: false,
                }],
                vec![],
            ],
            settings,
        };
        let key = side_key(&collection, &[track.clone()], 0);
        let mut renamed = collection.clone();
        renamed.name = "New name".into();
        renamed.sides[1] = collection.sides[0].clone();
        assert_eq!(side_key(&renamed, &[track.clone()], 0), key);
        renamed.settings.gap = 3.;
        assert_ne!(side_key(&renamed, &[track], 0), key);
    }
}
