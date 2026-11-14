//! Playback workflow and transport controls.
use super::*;
impl Studio {
    pub(super) fn stop(&mut self) {
        if matches!(self.dialog, Some(Dialog::PlaybackPreparing { .. })) {
            self.dialog = None;
        }
        self.preview.clear();
        self.player.stop();
        self.session = Default::default();
    }
    pub(super) fn preview_entry(&mut self, e: Entry) {
        if let Some(t) = self.library.tracks.iter().find(|t| t.id == e.track_id) {
            self.player.title = format!("{} — {}", t.title, t.artist);
            if e.duration <= 7200.0 && (e.offset > 0.0 || e.fade_in > 0.0 || e.fade_out > 0.0) {
                let mut s = self.session.settings.clone();
                s.side_seconds = e.duration.max(1.0);
                s.lead_in = 0.0;
                s.gap = 0.0;
                self.io.json(
                    Tag::EntryAudio {
                        generation: self.player.generation,
                        duration: e.duration,
                    },
                    "POST",
                    "/api/render",
                    json!({"sides":[[e],[]],"settings":s}),
                );
            } else {
                self.io.json(
                    Tag::Preview {
                        generation: self.player.generation,
                        offset: e.offset,
                        duration: e.duration,
                    },
                    "POST",
                    &format!("/api/preview/{}", t.id),
                    value(&self.session.settings),
                );
            }
        }
    }
    pub(super) fn play(&mut self) {
        self.stop();
        if self.is_tape() {
            let c = self.current().cloned().unwrap();
            self.session =
                crate::play_session::Session::side(&c.sides[self.side], c.settings.clone());
            if self
                .prepared_audio
                .as_ref()
                .is_some_and(|(id, side, snapshot, _)| {
                    *id == c.id
                        && *side == self.side
                        && *snapshot == render_key(&c, &self.library.tracks)
                })
            {
                let bytes = self.prepared_audio.as_ref().unwrap().3.clone();
                self.player.title = format!(
                    "{} / SIDE {}",
                    c.name,
                    if self.side == 0 { "A" } else { "B" }
                );
                if let Err(e) = self.player.play(bytes, 0., 0.) {
                    self.notify_error(e);
                }
            } else if let Some((_, urls)) = self
                .exports
                .get(&c.id)
                .filter(|(key, _)| *key == render_key(&c, &self.library.tracks))
            {
                self.player.title = format!(
                    "{} / SIDE {}",
                    c.name,
                    if self.side == 0 { "A" } else { "B" }
                );
                self.dialog = Some(Dialog::PlaybackPreparing {
                    generation: self.player.generation,
                    name: self.player.title.clone(),
                    loading: true,
                });
                self.io.json(
                    Tag::Audio {
                        generation: self.player.generation,
                        offset: 0.,
                        duration: 0.,
                    },
                    "GET",
                    &urls[self.side],
                    Value::Null,
                );
            } else {
                self.dialog = Some(Dialog::PlaybackPreparing {
                    generation: self.player.generation,
                    name: format!(
                        "{} / SIDE {}",
                        c.name,
                        if self.side == 0 { "A" } else { "B" }
                    ),
                    loading: false,
                });
                self.render(true);
            }
        } else {
            let entries = self.entries();
            let index = self.selected.first().copied().unwrap_or(0);
            let (entries, index) = if self.selected.len() > 1 {
                (
                    entries
                        .into_iter()
                        .enumerate()
                        .filter(|(i, _)| self.selected.contains(i))
                        .map(|(_, e)| e)
                        .collect(),
                    0,
                )
            } else {
                (entries, index)
            };
            self.session = crate::play_session::Session::queue(entries, index, self.config());
            if let Some(e) = self.session.entry() {
                self.preview_entry(e);
            }
        }
    }
    pub(super) fn render(&mut self, play: bool) {
        if let Some(c) = self.current().cloned() {
            self.io.json(
                Tag::Render {
                    snapshot: render_key(&c, &self.library.tracks),
                    id: c.id.clone(),
                    generation: self.player.generation,
                    play,
                    prepare: false,
                    side: self.side,
                },
                "POST",
                "/api/render",
                json!({"sides":c.sides,"settings":c.settings}),
            );
            self.player.title = format!(
                "{} / SIDE {}",
                c.name,
                if self.side == 0 { "A" } else { "B" }
            );
        }
    }
    pub(super) fn start_output(&mut self) {
        let Some(c) = self.current().cloned() else {
            return;
        };
        self.stop();
        let mut recording_player = Player::new();
        recording_player.volume = self.player.volume;
        #[cfg(not(target_arch = "wasm32"))]
        {
            recording_player.device = self.player.device.clone();
        }
        let snapshot = render_key(&c, &self.library.tracks);
        let urls = self
            .exports
            .get(&c.id)
            .filter(|(key, _)| *key == snapshot)
            .map(|(_, urls)| urls.clone());
        let ready = self
            .prepared_audio
            .as_ref()
            .is_some_and(|(id, side, key, _)| {
                *id == c.id && *side == self.side && *key == snapshot
            });
        self.dialog = Some(Dialog::Output(Box::new(crate::output::Output {
            id: c.id.clone(),
            name: c.name.clone(),
            side: self.side,
            snapshot: snapshot.clone(),
            urls: urls.clone(),
            error: None,
            notice: None,
            player: recording_player,
            countdown: None,
        })));
        if let Some(urls) = urls {
            if !ready {
                self.io.json(
                    Tag::DeckAudio {
                        id: c.id,
                        side: self.side,
                        snapshot,
                    },
                    "GET",
                    &urls[self.side],
                    Value::Null,
                );
            }
            return;
        }
        self.io.json(
            Tag::Render {
                id: c.id,
                generation: self.player.generation,
                play: false,
                prepare: true,
                side: self.side,
                snapshot,
            },
            "POST",
            "/api/render",
            json!({"sides":c.sides,"settings":c.settings}),
        );
    }
    pub(super) fn play_from(&mut self, index: usize) {
        self.stop();
        self.session = crate::play_session::Session::queue(self.entries(), index, self.config());
        if let Some(e) = self.session.entry() {
            self.preview_entry(e);
        }
    }
    pub(super) fn navigate(&mut self, delta: isize) {
        if !self.session.active() {
            let entries = self.entries();
            let index = self.selected.first().copied().unwrap_or(0);
            self.session = crate::play_session::Session::queue(entries, index, self.config());
        }
        match self.session.navigate(
            delta,
            self.session.pending_seek.unwrap_or(self.player.position()),
        ) {
            Some(crate::play_session::Target::Seek(at)) => {
                if self.player.length > 0. {
                    self.player.seek(at);
                } else {
                    self.session.pending_seek = Some(at);
                }
            }
            Some(crate::play_session::Target::Entry(entry)) => {
                self.preview.clear();
                self.player.stop();
                self.preview_entry(entry);
            }
            None => {}
        }
    }
    pub(super) fn playback_tick(&mut self, now: f64) {
        match self
            .session
            .tick(now, self.player.length > 0. && self.player.finished())
        {
            crate::play_session::Advance::Pause => self.player.pause(),
            crate::play_session::Advance::Play(e) => {
                self.player.stop();
                self.preview_entry(e);
            }
            crate::play_session::Advance::None => {}
        }
    }
    pub(super) fn player_ui(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("player")
            .min_height(80.)
            .frame(
                egui::Frame::new()
                    .fill(appearance::PAPER)
                    .inner_margin(egui::Margin::symmetric(24, 8)),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 4.;
                ui.horizontal(|ui| {
                    let width = ui.available_width();
                    let info_width = (width * 0.29).clamp(170., 350.);
                    let volume_width = 210.;
                    let transport_width = (width - info_width - volume_width - 32.).max(200.);
                    ui.allocate_ui_with_layout(
                        egui::vec2(info_width, 60.),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.set_min_size(egui::vec2(info_width, 60.));
                            let (r, _) =
                                ui.allocate_exact_size(egui::vec2(44., 44.), egui::Sense::hover());
                            ui.painter()
                                .rect_filled(r, 5., Color32::from_rgb(230, 233, 223));
                            ui.painter().text(
                                r.center(),
                                egui::Align2::CENTER_CENTER,
                                "♫",
                                egui::FontId::proportional(23.),
                                appearance::GREEN,
                            );
                            ui.vertical(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(if self.player.title.is_empty() {
                                            tr(
                                                self.ja,
                                                "再生する曲を選択",
                                                "Choose a track to play",
                                            )
                                        } else {
                                            &self.player.title
                                        })
                                        .strong()
                                        .size(12.),
                                    )
                                    .truncate(),
                                );
                                ui.label(
                                    RichText::new("Cassette Studio")
                                        .small()
                                        .color(appearance::MUTED),
                                );
                            });
                        },
                    );
                    ui.allocate_ui_with_layout(
                        egui::vec2(transport_width, 60.),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.set_min_size(egui::vec2(transport_width, 60.));
                            ui.horizontal(|ui| {
                                ui.add_space(((transport_width - 180.) / 2.).max(0.));
                                if ui
                                    .add_sized([32., 36.], egui::Button::new("⏮").frame(false))
                                    .on_hover_text(tr(self.ja, "前の曲", "Previous track"))
                                    .clicked()
                                {
                                    self.navigate(-1);
                                }
                                let playing = self.player.length > 0. && !self.player.paused();
                                if ui
                                    .add_sized(
                                        [36., 36.],
                                        egui::Button::new(
                                            RichText::new(if playing { "⏸" } else { "▶" })
                                                .color(Color32::WHITE),
                                        )
                                        .fill(appearance::GREEN)
                                        .corner_radius(18.),
                                    )
                                    .on_hover_text(tr(self.ja, "再生 / 一時停止", "Play / pause"))
                                    .clicked()
                                {
                                    if self.player.length > 0.
                                        && self.player.position() < self.player.length - 0.02
                                    {
                                        self.player.toggle();
                                    } else {
                                        self.play();
                                    }
                                }
                                if ui
                                    .add_sized([32., 36.], egui::Button::new("■").frame(false))
                                    .on_hover_text(tr(self.ja, "停止", "Stop"))
                                    .clicked()
                                {
                                    self.stop();
                                }
                                if ui
                                    .add_sized([32., 36.], egui::Button::new("⏭").frame(false))
                                    .on_hover_text(tr(self.ja, "次の曲", "Next track"))
                                    .clicked()
                                {
                                    self.navigate(1);
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.visuals_mut().widgets.inactive.bg_fill = appearance::LINE;
                                ui.visuals_mut().widgets.active.bg_fill = appearance::GREEN;
                                let mut position = self.player.position();
                                ui.small(time(position));
                                ui.spacing_mut().slider_width = (transport_width - 104.).max(90.);
                                if ui
                                    .add(crate::appearance::slider(
                                        egui::Slider::new(
                                            &mut position,
                                            0.0..=self.player.length.max(1.),
                                        )
                                        .show_value(false),
                                    ))
                                    .changed()
                                {
                                    self.player.seek(position);
                                }
                                ui.small(format!(
                                    "−{}",
                                    time((self.player.length - position).max(0.))
                                ));
                            });
                        },
                    );
                    ui.allocate_ui_with_layout(
                        egui::vec2(volume_width, 60.),
                        egui::Layout::top_down(egui::Align::Max),
                        |ui| {
                            ui.set_min_size(egui::vec2(volume_width, 60.));
                            ui.horizontal(|ui| {
                                ui.visuals_mut().widgets.inactive.bg_fill = appearance::LINE;
                                ui.visuals_mut().widgets.active.bg_fill = appearance::GREEN;
                                ui.small(tr(self.ja, "音量", "Volume"));
                                ui.spacing_mut().slider_width = 110.;
                                let mut volume = self.player.volume;
                                if ui
                                    .add(crate::appearance::slider(
                                        egui::Slider::new(&mut volume, 0.0..=1.).show_value(false),
                                    ))
                                    .changed()
                                {
                                    self.player.set_volume(volume);
                                }
                            });
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                let before = self.player.device.clone();
                                let mut selected = before.clone();
                                egui::ComboBox::from_id_salt("output-device")
                                    .selected_text(selected.as_deref().unwrap_or(tr(
                                        self.ja,
                                        "出力先：既定",
                                        "Output: default",
                                    )))
                                    .width(160.)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut selected,
                                            None,
                                            tr(self.ja, "既定", "Default"),
                                        );
                                        for d in Player::devices() {
                                            ui.selectable_value(&mut selected, Some(d.clone()), d);
                                        }
                                    });
                                if selected != before {
                                    self.stop();
                                    self.player.change_output(selected);
                                }
                            }
                        },
                    );
                });
            });
    }
}
