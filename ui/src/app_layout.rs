//! Main toolbar, library navigation and collection headings.
use super::*;
impl Studio {
    pub(super) fn top(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("main-toolbar")
            .frame(
                egui::Frame::new()
                    .fill(appearance::PAPER)
                    .inner_margin(egui::Margin::symmetric(28, 8)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    appearance::brand(ui);
                    appearance::right_space(
                        ui,
                        &[
                            tr(self.ja, "＋ ファイル追加", "＋ Add files"),
                            tr(self.ja, "↗ URL追加", "↗ Add URL"),
                            tr(self.ja, "オプション", "Options"),
                        ],
                    );
                    ui.add_enabled_ui(!self.io.busy() && !self.feedback.importing(), |ui| {
                        if ui
                            .add(appearance::primary(tr(
                                self.ja,
                                "＋ ファイル追加",
                                "＋ Add files",
                            )))
                            .clicked()
                        {
                            platform::pick(platform::Pick::Media, self.io.tx.clone(), ctx.clone());
                        }
                        if ui.button(tr(self.ja, "↗ URL追加", "↗ Add URL")).clicked() {
                            self.dialog = Some(Dialog::Url(String::new()));
                        }
                        if ui.button(tr(self.ja, "オプション", "Options")).clicked() {
                            self.dialog = Some(Dialog::Options);
                        }
                    });
                    if self.io.busy() && !self.feedback.importing() {
                        ui.spinner();
                        ui.label(tr(self.ja, "処理中…", "Working…"));
                    }
                });
            });
    }
    pub(super) fn library_panel(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("library")
            .resizable(true)
            .default_width(250.)
            .width_range(170.0..=500.)
            .frame(
                egui::Frame::new()
                    .fill(appearance::SIDEBAR)
                    .inner_margin(14.),
            )
            .show(ctx, |ui| {
                ui.heading(tr(self.ja, "ライブラリ", "Library"));
                if ui
                    .selectable_label(
                        self.active.is_none(),
                        tr(self.ja, "すべての曲", "All tracks"),
                    )
                    .clicked()
                {
                    self.select(None);
                }
                ui.horizontal(|ui| {
                    if ui
                        .small_button(tr(self.ja, "＋プレイリスト", "+ Playlist"))
                        .clicked()
                    {
                        self.make_name("playlist", false);
                    }
                    if ui
                        .small_button(tr(self.ja, "＋空カセット", "+ Blank tape"))
                        .clicked()
                    {
                        self.make_name("blank", false);
                    }
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for kind in ["playlist", "cassette"] {
                        ui.separator();
                        ui.strong(if kind == "playlist" {
                            "PLAYLISTS"
                        } else {
                            "CASSETTES"
                        });
                        let collections = self
                            .library
                            .collections
                            .iter()
                            .filter(|c| c.kind == kind)
                            .cloned()
                            .collect::<Vec<_>>();
                        for c in collections {
                            let selected = self.active.as_ref() == Some(&c.id);
                            let row = egui::Frame::new()
                                .fill(if selected {
                                    Color32::from_rgb(220, 233, 224)
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .corner_radius(6.)
                                .inner_margin(8.)
                                .show(ui, |ui| {
                                    ui.set_width((ui.available_width() - 16.).max(100.));
                                    ui.horizontal(|ui| {
                                        if kind == "cassette" {
                                            appearance::cassette(ui);
                                        } else {
                                            ui.label(
                                                RichText::new("♫")
                                                    .size(24.)
                                                    .color(appearance::GREEN),
                                            );
                                        }
                                        ui.vertical(|ui| {
                                            ui.add(
                                                egui::Label::new(
                                                    RichText::new(&c.name).color(appearance::GREEN),
                                                )
                                                .truncate(),
                                            );
                                            ui.label(
                                                RichText::new(if kind == "cassette" {
                                                    format!("{} × 2", time(c.settings.side_seconds))
                                                } else {
                                                    format!(
                                                        "{} {}",
                                                        c.tracks.len(),
                                                        tr(self.ja, "曲", "tracks")
                                                    )
                                                })
                                                .size(12.)
                                                .color(appearance::MUTED),
                                            );
                                        });
                                    });
                                })
                                .response;
                            if ui
                                .interact(row.rect, row.id, egui::Sense::click())
                                .clicked()
                            {
                                self.select(Some(c.id));
                            }
                        }
                    }
                });
            });
    }
    pub(super) fn collection_heading(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new(if self.is_tape() {
                "YOUR PERSONAL MIXTAPE"
            } else if self.current().is_some() {
                "PLAYLIST"
            } else {
                "YOUR MUSIC COLLECTION"
            })
            .size(12.)
            .strong()
            .color(appearance::GREEN),
        );
        ui.horizontal_wrapped(|ui| {
            ui.heading(
                RichText::new(
                    self.current()
                        .map(|c| c.name.clone())
                        .unwrap_or_else(|| tr(self.ja, "すべての曲", "All tracks").into()),
                )
                .size(21.)
                .strong(),
            );
            let mut actions = Vec::new();
            if self.current().is_some() {
                actions.push(tr(self.ja, "ライブラリから追加", "Add from library"));
                if self.is_tape() {
                    actions.push(tr(self.ja, "⚙ 録音設定", "⚙ Recording settings"));
                }
                actions.extend([
                    tr(self.ja, "名前変更", "Rename"),
                    tr(self.ja, "削除", "Delete"),
                ]);
            } else {
                actions.push(tr(
                    self.ja,
                    "⚙ 作成・取り込み設定",
                    "⚙ Creation / import settings",
                ));
            }
            if !self.is_tape() {
                actions.extend([
                    tr(self.ja, "選択／曲順で作成", "Create from selection / order"),
                    tr(
                        self.ja,
                        "✦ 自動でカセット作成",
                        "✦ Create cassette automatically",
                    ),
                ]);
            }
            appearance::right_space(ui, &actions);
            if self.current().is_some() {
                if ui
                    .button(tr(self.ja, "ライブラリから追加", "Add from library"))
                    .clicked()
                {
                    self.dialog = Some(Dialog::Add(BTreeSet::new()));
                }
                if self.is_tape()
                    && ui
                        .button(tr(self.ja, "⚙ 録音設定", "⚙ Recording settings"))
                        .clicked()
                {
                    self.dialog = Some(Dialog::Settings {
                        target: self.active.clone(),
                        value: self.config(),
                    });
                }
                if ui.button(tr(self.ja, "名前変更", "Rename")).clicked() {
                    self.dialog = Some(Dialog::Rename(self.current().unwrap().name.clone()));
                }
                if ui
                    .add(appearance::danger(tr(self.ja, "削除", "Delete")))
                    .clicked()
                {
                    self.dialog = Some(Dialog::Delete { collection: true });
                }
            } else if ui
                .button(tr(
                    self.ja,
                    "⚙ 作成・取り込み設定",
                    "⚙ Creation / import settings",
                ))
                .clicked()
            {
                self.dialog = Some(Dialog::Settings {
                    target: None,
                    value: self.defaults.clone(),
                });
            }
            if !self.is_tape() {
                if ui
                    .button(tr(
                        self.ja,
                        "選択／曲順で作成",
                        "Create from selection / order",
                    ))
                    .clicked()
                {
                    self.make_name("cassette", false);
                }
                if appearance::automatic_button(
                    ui,
                    tr(
                        self.ja,
                        "✦ 自動でカセット作成",
                        "✦ Create cassette automatically",
                    ),
                )
                .clicked()
                {
                    let blanks = self
                        .library
                        .collections
                        .iter()
                        .filter(|c| c.kind == "cassette" && c.sides.iter().all(Vec::is_empty))
                        .map(|c| c.id.clone())
                        .collect();
                    if self
                        .library
                        .collections
                        .iter()
                        .any(|c| c.kind == "cassette" && c.sides.iter().all(Vec::is_empty))
                    {
                        self.dialog = Some(Dialog::Batch(blanks));
                    } else {
                        self.make_name("cassette", true);
                    }
                }
            }
        });
        let summary = if let Some(c) = self.current().filter(|c| c.kind == "cassette") {
            format!(
                "{} × {} / {} {:.1}{} / {}",
                time(c.settings.side_seconds),
                tr(self.ja, "2面", "2 sides"),
                tr(self.ja, "曲間", "Gap"),
                c.settings.gap,
                tr(self.ja, "秒", "s"),
                if c.settings.continuation == "resume" {
                    tr(
                        self.ja,
                        "引き継ぎ：途中からフェードイン",
                        "Continuation: resume with fade-in",
                    )
                } else {
                    tr(self.ja, "引き継ぎ：最初から", "Continuation: restart")
                }
            )
        } else {
            let entries = self.entries();
            format!(
                "{}{} · {} {}",
                entries.len(),
                tr(self.ja, "曲", " tracks"),
                tr(self.ja, "無音除去後", "After silence trimming"),
                time(entries.iter().map(|e| e.duration).sum())
            )
        };
        ui.label(RichText::new(summary).small().color(appearance::MUTED));
        ui.add_space(6.);
        if self.is_tape() {
            let tape = self.current().cloned().unwrap();
            ui.columns(2, |columns| {
                for (side, column) in columns.iter_mut().enumerate() {
                    let used =
                        tape.settings.lead_in + side_duration(&tape.sides[side], tape.settings.gap);
                    let remaining = tape.settings.side_seconds - used;
                    let frame = egui::Frame::new()
                        .fill(appearance::PAPER)
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            if self.side == side {
                                appearance::GREEN
                            } else {
                                appearance::LINE
                            },
                        ))
                        .corner_radius(6.)
                        .inner_margin(10.)
                        .show(column, |ui| {
                            ui.set_min_width((ui.available_width() - 20.).max(100.));
                            ui.horizontal(|ui| {
                                ui.small(format!(
                                    "SIDE {} · {} TRACKS",
                                    if side == 0 { "A" } else { "B" },
                                    tape.sides[side].len()
                                ));
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        ui.small(format!(
                                            "{} / {}",
                                            time(used),
                                            time(tape.settings.side_seconds)
                                        ));
                                    },
                                );
                            });
                            ui.add(
                                egui::ProgressBar::new(
                                    (used / tape.settings.side_seconds).clamp(0., 1.) as f32,
                                )
                                .fill(if remaining < 0. {
                                    Color32::from_rgb(189, 88, 76)
                                } else {
                                    appearance::GREEN
                                })
                                .desired_height(5.),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "{} {}",
                                    if remaining < 0. {
                                        tr(self.ja, "時間超過", "Over capacity")
                                    } else {
                                        tr(self.ja, "残り", "Remaining")
                                    },
                                    time(remaining.abs())
                                ))
                                .small()
                                .color(if remaining < 0. {
                                    Color32::from_rgb(189, 88, 76)
                                } else {
                                    appearance::MUTED
                                }),
                            );
                        })
                        .response;
                    if column
                        .interact(frame.rect, frame.id, egui::Sense::click())
                        .clicked()
                    {
                        self.side = side;
                        self.selected.clear();
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                appearance::right_space(
                    ui,
                    &[
                        tr(self.ja, "J-CARDを編集", "Edit J-CARD"),
                        tr(self.ja, "A/B面のLUFSグラフ", "A/B-side LUFS graph"),
                        tr(
                            self.ja,
                            "WAV出力・デッキ録音",
                            "WAV export / deck recording",
                        ),
                    ],
                );
                if ui
                    .button(tr(self.ja, "J-CARDを編集", "Edit J-CARD"))
                    .clicked()
                {
                    let id = self.active.clone().unwrap();
                    self.io.json(
                        Tag::JCard(id.clone()),
                        "GET",
                        &format!("/api/jcards/{id}"),
                        Value::Null,
                    );
                }
                if ui
                    .button(tr(self.ja, "A/B面のLUFSグラフ", "A/B-side LUFS graph"))
                    .clicked()
                {
                    self.side_graph();
                }
                if ui
                    .add(appearance::primary(tr(
                        self.ja,
                        "WAV出力・デッキ録音",
                        "WAV export / deck recording",
                    )))
                    .clicked()
                {
                    self.start_output();
                }
            });
            let c = self.current().unwrap();
            let used = c.settings.lead_in + side_duration(&c.sides[self.side], c.settings.gap);
            ui.label(format!(
                "{} / {}   {} {}",
                time(used),
                time(c.settings.side_seconds),
                tr(self.ja, "残り", "Remaining"),
                time(c.settings.side_seconds - used)
            ));
        }
    }
}
