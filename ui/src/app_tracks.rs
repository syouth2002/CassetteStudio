//! Track rows, selection, context menus and graphs.
use super::*;
impl Studio {
    pub(super) fn track_controls(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = 4.;
        ui.horizontal_wrapped(|ui| {
            if self.is_tape() {
                for side in 0..2 {
                    if ui
                        .selectable_label(
                            self.side == side,
                            if side == 0 { "SIDE A" } else { "SIDE B" },
                        )
                        .clicked()
                    {
                        self.side = side;
                        self.selected.clear();
                    }
                }
            }
            ui.label(format!(
                "{} {}",
                self.selected.len(),
                tr(self.ja, "曲選択", "selected")
            ));
        });
        ui.separator();

        let status_width = (ui.available_width() * 0.27).clamp(180., 280.);
        let move_width = if self.active.is_some() { 52. } else { 0. };
        let title_width = (ui.available_width() - status_width - move_width - 240.).max(80.);
        ui.horizontal(|ui| {
            appearance::header_cell(ui, 24., |ui| {
                let count = self.entries_len();
                let mut all = count > 0 && self.selected.len() == count;
                let partial = !self.selected.is_empty() && !all;
                if ui
                    .add(egui::Checkbox::without_text(&mut all).indeterminate(partial))
                    .on_hover_text(tr(self.ja, "全て選択 / 選択解除", "Select / clear all"))
                    .changed()
                {
                    self.selected = if all {
                        (0..count).collect()
                    } else {
                        BTreeSet::new()
                    };
                }
            });
            appearance::header_cell(ui, 28., |ui| {
                ui.small("#");
            });
            appearance::header_cell(ui, title_width, |ui| {
                ui.small(tr(self.ja, "曲名／アーティスト", "Title / artist"));
            });
            appearance::header_cell(ui, 68., |ui| {
                ui.small(tr(self.ja, "再生時間", "Duration"));
            });
            appearance::header_cell(ui, 68., |ui| {
                ui.small(tr(self.ja, "元の時間", "Original"));
            });
            appearance::header_cell(ui, status_width, |ui| {
                ui.small(tr(self.ja, "状態", "Status"));
            });
        });
        ui.separator();
    }
    pub(super) fn track_rows(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.y = 4.;
        let status_width = (ui.available_width() * 0.27).clamp(180., 280.);
        let move_width = if self.active.is_some() { 52. } else { 0. };
        let title_width = (ui.available_width() - status_width - move_width - 240.).max(80.);

        let entries = self.entries();
        let mut reorder = None;
        let mut row_rects = Vec::new();
        let mut end_boundary = 0.;
        for (i, e) in entries.iter().enumerate() {
            let Some(t) = self
                .library
                .tracks
                .iter()
                .find(|t| t.id == e.track_id)
                .cloned()
            else {
                continue;
            };
            ui.push_id(i, |ui| {
                let row =
                    ui.scope_builder(egui::UiBuilder::new().sense(egui::Sense::click()), |ui| {
                        ui.horizontal(|ui| {
                            let mut checked = self.selected.contains(&i);
                            if ui.checkbox(&mut checked, "").changed() {
                                if checked {
                                    self.selected.insert(i);
                                } else {
                                    self.selected.remove(&i);
                                }
                            }
                            appearance::column(ui, egui::vec2(28., 40.), |ui| {
                                ui.label(
                                    RichText::new(format!("{:02}", i + 1)).color(appearance::MUTED),
                                );
                            });
                            let (rect, r) = ui.allocate_exact_size(
                                egui::vec2(title_width, 40.),
                                egui::Sense::click_and_drag(),
                            );
                            if checked || r.hovered() {
                                ui.painter().rect_filled(
                                    rect,
                                    4.,
                                    if checked {
                                        Color32::from_rgb(220, 233, 224)
                                    } else {
                                        Color32::from_rgb(238, 240, 232)
                                    },
                                );
                            }
                            let mut name_ui = ui.new_child(
                                egui::UiBuilder::new()
                                    .max_rect(rect.shrink2(egui::vec2(6., 2.)))
                                    .layout(egui::Layout::top_down(egui::Align::Min)),
                            );

                            name_ui.spacing_mut().item_spacing.y = 2.;
                            name_ui.add(
                                egui::Label::new(RichText::new(&t.title).strong())
                                    .truncate()
                                    .selectable(false),
                            );
                            name_ui.add(
                                egui::Label::new(
                                    RichText::new(if t.artist.is_empty() {
                                        &t.filename
                                    } else {
                                        &t.artist
                                    })
                                    .small()
                                    .color(appearance::MUTED),
                                )
                                .truncate()
                                .selectable(false),
                            );
                            if r.clicked() {
                                if !ui.input(|i| i.modifiers.command || i.modifiers.shift) {
                                    self.selected.clear();
                                }
                                self.selected.insert(i);
                            }
                            if r.secondary_clicked() && !self.selected.contains(&i) {
                                self.selected.clear();
                                self.selected.insert(i);
                            }
                            r.context_menu(|ui| self.track_menu(ui, i));
                            if r.double_clicked() {
                                self.play_from(i);
                            }
                            if self.active.is_some() && r.drag_started() {
                                r.dnd_set_drag_payload(crate::track_drag::TrackDrag {
                                    collection: self.active.clone().unwrap(),
                                    side: self.side,
                                    from: i,
                                    title: t.title.clone(),
                                    artist: if t.artist.is_empty() {
                                        t.filename.clone()
                                    } else {
                                        t.artist.clone()
                                    },
                                });
                            }
                            appearance::column(ui, egui::vec2(68., 40.), |ui| {
                                ui.label(time(e.duration));
                            });
                            appearance::column(ui, egui::vec2(68., 40.), |ui| {
                                ui.label(RichText::new(time(t.duration)).color(appearance::MUTED));
                            });
                            let status =
                                appearance::column(ui, egui::vec2(status_width, 40.), |ui| {
                                    appearance::badges(
                                        ui,
                                        &appearance::compact_status(
                                            &t,
                                            e,
                                            &self.config(),
                                            self.ja,
                                            !self.is_tape(),
                                        ),
                                    );
                                });
                            status.response.on_hover_ui(|ui| {
                                ui.set_max_width(360.);
                                for label in
                                    appearance::row_status(&t, e, &self.config(), self.ja, true)
                                {
                                    ui.label(label);
                                }
                                ui.small(tr(
                                    self.ja,
                                    "詳細は右クリック → プロパティ",
                                    "More: right-click → Properties",
                                ));
                            });
                            if self.active.is_some() {
                                if ui.small_button("↑").clicked() {
                                    self.move_entry(i, -1);
                                }
                                if ui.small_button("↓").clicked() {
                                    self.move_entry(i, 1);
                                }
                            }
                        });
                    });
                if row.response.secondary_clicked() && !self.selected.contains(&i) {
                    self.selected.clear();
                    self.selected.insert(i);
                }
                row.response.context_menu(|ui| self.track_menu(ui, i));
                row_rects.push((i, row.response.rect));
                end_boundary = ui.separator().rect.center().y;
            });
        }

        if let Some(drag) = egui::DragAndDrop::payload::<crate::track_drag::TrackDrag>(ui.ctx()) {
            let same_list =
                self.active.as_ref() == Some(&drag.collection) && self.side == drag.side;
            let mut destination = None;
            if same_list {
                if let Some((_, rect)) = row_rects.iter().find(|(i, _)| *i == drag.from) {
                    ui.painter().rect_stroke(
                        *rect,
                        4.,
                        egui::Stroke::new(2_f32, appearance::GREEN),
                        egui::StrokeKind::Inside,
                    );
                }
                if let Some(pointer) = ui
                    .ctx()
                    .pointer_hover_pos()
                    .filter(|p| ui.clip_rect().contains(*p))
                {
                    let boundary = crate::track_drag::boundaries(&row_rects, end_boundary)
                        .into_iter()
                        .min_by(|a, b| (a.1 - pointer.y).abs().total_cmp(&(b.1 - pointer.y).abs()));
                    if let Some((boundary, y)) = boundary {
                        destination =
                            crate::track_drag::destination(drag.from, boundary, entries.len());
                        if destination.is_some() {
                            let left = ui.clip_rect().left() + 5.;
                            let right = ui.clip_rect().right() - 5.;
                            ui.painter().line_segment(
                                [egui::pos2(left, y), egui::pos2(right, y)],
                                egui::Stroke::new(3_f32, appearance::GREEN),
                            );
                            for x in [left, right] {
                                ui.painter()
                                    .circle_filled(egui::pos2(x, y), 4., appearance::GREEN);
                            }
                        }
                    }
                }
            }
            if same_list
                && ui.input(|i| i.pointer.primary_down())
                && let Some(pointer) = ui.ctx().pointer_hover_pos()
            {
                let speed = crate::track_drag::edge_scroll(pointer, ui.clip_rect());
                if speed != 0. {
                    let dt = ui.input(|i| i.stable_dt).min(0.05);
                    ui.scroll_with_delta_animation(
                        egui::vec2(0., speed * dt),
                        egui::style::ScrollAnimation::none(),
                    );
                    ui.ctx().request_repaint();
                }
            }
            if ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary)) {
                if let Some(to) = destination {
                    reorder = Some((drag.from, to));
                }
                egui::DragAndDrop::clear_payload(ui.ctx());
            } else {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                crate::track_drag::preview(ui.ctx(), &drag, destination, self.ja);
            }
        }

        if let Some((from, to)) = reorder
            && from != to
            && let Some(mut c) = self.current().cloned()
        {
            if c.kind == "cassette" {
                let e = c.sides[self.side].remove(from);
                c.sides[self.side].insert(to, e);
            } else {
                let e = c.tracks.remove(from);
                c.tracks.insert(to, e);
            }
            self.selected.clear();
            self.selected.insert(to);
            self.save_collection_request(c, Tag::Reordered);
        }
    }
    pub(super) fn tracks(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(246, 244, 239))
                    .inner_margin(egui::Margin::symmetric(24, 20)),
            )
            .show(ctx, |ui| {
                let scroll_id = egui::Id::new(("workspace-scroll", self.active.clone()));
                egui::ScrollArea::vertical()
                    .id_salt(scroll_id)
                    .auto_shrink([false, false])
                    .show_viewport(ui, |ui, _viewport| {
                        self.collection_heading(ui);
                        let natural = ui.next_widget_position();
                        let clip = ui.clip_rect();
                        let width = ui.available_width();
                        let sticky_y = natural.y.max(clip.top());
                        let height_id = egui::Id::new(("track-controls-height", self.is_tape()));
                        let height = ctx.data(|d| d.get_temp::<f32>(height_id)).unwrap_or(88.);
                        ui.allocate_space(egui::vec2(width, height));
                        ui.scope(|ui| {
                            let rows_clip = egui::Rect::from_min_max(
                                egui::pos2(clip.left(), (sticky_y + height).min(clip.bottom())),
                                clip.max,
                            );
                            ui.set_clip_rect(clip.intersect(rows_clip));
                            self.track_rows(ui);
                        });
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(natural.x, sticky_y),
                            egui::vec2(width, height),
                        );
                        ui.painter()
                            .rect_filled(rect, 0., Color32::from_rgb(246, 244, 239));
                        let mut sticky = ui.new_child(
                            egui::UiBuilder::new()
                                .id_salt("sticky-track-controls")
                                .max_rect(rect)
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        );
                        sticky.set_clip_rect(clip);
                        self.track_controls(&mut sticky);
                        let measured = sticky.min_rect().height() + ui.spacing().item_spacing.y;
                        if (measured - height).abs() > 0.5 {
                            ctx.data_mut(|d| d.insert_temp(height_id, measured));
                            ctx.request_repaint();
                        }
                    });
            });
    }
    pub(super) fn selected_track(&self) -> Option<Track> {
        let e = self.entries().get(*self.selected.first()?).cloned()?;
        self.library
            .tracks
            .iter()
            .find(|t| t.id == e.track_id)
            .cloned()
    }
    pub(super) fn properties(&mut self) {
        if let Some(track) = self.selected_track() {
            let id = track.id.clone();
            let entry = if self.is_tape() {
                self.selected
                    .first()
                    .and_then(|i| self.entries().get(*i).cloned())
            } else {
                None
            };
            self.dialog = Some(Dialog::Properties(Box::new(
                crate::properties::Properties {
                    track,
                    entry,
                    settings: self.config(),
                    media: None,
                    error: None,
                },
            )));
            self.io.json(
                Tag::Properties(id.clone()),
                "GET",
                &format!("/api/tracks/{id}/properties"),
                Value::Null,
            );
        }
    }
    pub(super) fn track_menu(&mut self, ui: &mut egui::Ui, index: usize) {
        let ja = self.ja;
        let single = self.selected.len() == 1;
        if ui
            .add_enabled(
                single && !self.io.busy(),
                egui::Button::new(tr(ja, "再生", "Play")),
            )
            .clicked()
        {
            self.play_from(index);
            ui.close();
        }
        if ui
            .add_enabled(
                self.player.length > 0.,
                egui::Button::new(tr(ja, "停止", "Stop")),
            )
            .clicked()
        {
            self.stop();
            ui.close();
        }
        ui.separator();
        ui.add_enabled_ui(!self.io.busy(), |ui| {
            if ui
                .add_enabled(
                    single,
                    egui::Button::new(
                        if self.selected_track().is_some_and(|t| t.mono == Some(true)) {
                            tr(ja, "音量調整・疑似ステレオ", "Loudness / pseudo stereo")
                        } else {
                            tr(ja, "音量調整", "Adjust loudness")
                        },
                    ),
                )
                .clicked()
            {
                if let Some(t) = self.selected_track() {
                    self.dialog = Some(Dialog::Gain {
                        gain: t.manual_gain_db,
                        strength: t.pseudo_stereo,
                        track: Box::new(t),
                    });
                }
                ui.close();
            }
            if ui
                .add_enabled(
                    single,
                    egui::Button::new(tr(ja, "LUFSグラフ", "LUFS graph")),
                )
                .clicked()
            {
                self.track_graph();
                ui.close();
            }
            if ui
                .button(tr(
                    ja,
                    "選択曲の音量・DC解析",
                    "Analyze selected loudness / DC",
                ))
                .clicked()
            {
                self.io.json(
                    Tag::Message,
                    "POST",
                    "/api/loudness",
                    json!({"tracks":self.ids(),"settings":self.config()}),
                );
                ui.close();
            }
            if ui
                .button(tr(ja, "選択曲の無音を再解析", "Reanalyze selected silence"))
                .clicked()
            {
                self.io.json(
                    Tag::Changed(None),
                    "PUT",
                    "/api/reanalyze",
                    json!({"tracks":self.ids(),"settings":self.config()}),
                );
                ui.close();
            }
            if self.is_tape() {
                ui.separator();
                if ui
                    .add_enabled(
                        single,
                        egui::Button::new(tr(ja, "再生範囲・フェード", "Playback range / fades")),
                    )
                    .clicked()
                {
                    if let Some(e) = self.entries().get(index).cloned() {
                        self.dialog = Some(Dialog::Entry { index, value: e });
                    }
                    ui.close();
                }
                if ui
                    .button(tr(ja, "反対面へ移動", "Move to opposite side"))
                    .clicked()
                {
                    self.move_opposite();
                    ui.close();
                }
            }
            if self.active.is_some() {
                if ui
                    .add_enabled(
                        single && index > 0,
                        egui::Button::new(tr(ja, "上へ移動", "Move up")),
                    )
                    .clicked()
                {
                    self.move_entry(index, -1);
                    ui.close();
                }
                if ui
                    .add_enabled(
                        single && index + 1 < self.entries_len(),
                        egui::Button::new(tr(ja, "下へ移動", "Move down")),
                    )
                    .clicked()
                {
                    self.move_entry(index, 1);
                    ui.close();
                }
            }
            ui.separator();
            if ui
                .add_enabled(
                    single,
                    egui::Button::new(tr(ja, "プロパティ", "Properties")),
                )
                .clicked()
            {
                self.properties();
                ui.close();
            }
            ui.separator();
            let label = if self.active.is_none() {
                tr(ja, "ライブラリから削除", "Delete from library")
            } else if self.is_tape() {
                tr(ja, "ミックスリストから削除", "Remove from mix")
            } else {
                tr(ja, "プレイリストから削除", "Remove from playlist")
            };
            if ui.add(appearance::danger(label)).clicked() {
                if self.active.is_none() {
                    self.dialog = Some(Dialog::Delete { collection: false });
                } else {
                    self.remove_selection();
                }
                ui.close();
            }
        });
    }
    pub(super) fn track_graph(&mut self) {
        if let Some(track) = self.selected_track() {
            let entry = if self.is_tape() {
                self.entries().get(*self.selected.first().unwrap()).cloned()
            } else {
                None
            };
            let settings = if self.is_tape() {
                self.config()
            } else {
                self.config().loudness_preview()
            };
            self.graph.normalization_preview = !self.is_tape();
            self.graph.collection = None;
            self.graph.title = track.title.clone();
            self.graph.target_lufs = settings.target_lufs;
            if let Some(generation) =
                self.graph
                    .request(crate::graph::track_key(&track, &settings, entry.as_ref()))
            {
                self.io.json(
                    Tag::Graph(generation),
                    "POST",
                    &format!("/api/tracks/{}/loudness-graph", track.id),
                    json!({"settings":settings,"entry":entry}),
                );
            }
        }
    }
    pub(super) fn side_graph(&mut self) {
        if let Some(id) = self.active.clone() {
            self.side_graph_for(id, self.side);
        }
    }
    pub(super) fn side_graph_for(&mut self, id: String, side: usize) {
        if let Some(collection) = self
            .library
            .collections
            .iter()
            .find(|c| c.id == id)
            .cloned()
        {
            self.graph.normalization_preview = false;
            self.graph.collection = Some(id.clone());
            self.graph.side = side;
            self.graph.title = format!(
                "{} / SIDE {}",
                collection.name,
                if side == 0 { "A" } else { "B" }
            );
            self.graph.target_lufs = collection.settings.target_lufs;
            if let Some(generation) = self.graph.request(crate::graph::side_key(
                &collection,
                &self.library.tracks,
                side,
            )) {
                self.io.json(
                    Tag::Graph(generation),
                    "POST",
                    &format!("/api/collections/{id}/loudness-graph"),
                    json!({"side":side}),
                );
            }
        }
    }
}
