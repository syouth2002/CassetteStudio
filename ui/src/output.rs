//! A rendering dialog owns the cassette identity and generated side links.
use crate::{
    appearance,
    settings::tr,
    transport::{Tag, Transport},
};
use eframe::egui;
use serde_json::Value;
pub struct Output {
    pub id: String,
    pub name: String,
    pub side: usize,
    pub snapshot: Value,
    pub urls: Option<[String; 2]>,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub player: crate::playback::Player,
    pub countdown: Option<f64>,
}
pub enum OutputAction {
    None,
    Start,
    Countdown,
    Side(usize),
    Toggle,
    Stop,
    Seek(f64),
    Volume(f32),
}
impl Output {
    pub fn stop(&mut self) {
        self.countdown = None;
        self.player.stop();
    }
    pub fn start(&mut self, bytes: Vec<u8>) {
        self.stop();
        self.error = None;
        if let Err(error) = self.player.play(bytes, 0., 0.) {
            self.error = Some(error);
        }
    }
    pub fn matches(&self, id: &str, snapshot: &Value) -> bool {
        self.id == id && self.snapshot == *snapshot
    }
    pub fn ui(
        &self,
        ui: &mut egui::Ui,
        io: &mut Transport,
        now: f64,
        ready: bool,
        ja: bool,
    ) -> OutputAction {
        let player = &self.player;
        let remaining = self.countdown.map(|deadline| (deadline - now).max(0.));
        ui.strong(&self.name);
        if let Some(error) = &self.error {
            ui.colored_label(appearance::RED, error);
            if self.urls.is_none() {
                return OutputAction::None;
            }
        }
        let Some(urls) = &self.urls else {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr(
                    ja,
                    "A・B面のWAVを生成しています…",
                    "Generating WAV audio for sides A and B…",
                ));
            });
            ui.label(tr(
                ja,
                "曲間・フェード・音量設定を反映しています。完了するまでお待ちください。",
                "Applying gaps, fades and level settings. Please wait until generation finishes.",
            ));
            return OutputAction::None;
        };
        ui.colored_label(
            appearance::GREEN,
            self.notice.as_deref().unwrap_or(tr(
                ja,
                "WAVの生成が完了しました",
                "WAV generation complete",
            )),
        );
        ui.separator();
        ui.strong(tr(ja, "WAVをダウンロード", "Download WAV files"));
        ui.add_enabled_ui(!io.busy(), |ui| {
            for (side, url) in urls.iter().enumerate() {
                let letter = if side == 0 { "A" } else { "B" };
                let label = if ja {
                    format!("↓ {letter}面のWAVを保存")
                } else {
                    format!("↓ Save side {letter} WAV")
                };
                if ui
                    .add(egui::Link::new(
                        egui::RichText::new(label)
                            .strong()
                            .underline()
                            .color(egui::Color32::from_rgb(30, 91, 150))
                            .size(16.),
                    ))
                    .clicked()
                {
                    io.json(
                        Tag::Download(format!("{}-{letter}.wav", self.name)),
                        "GET",
                        url,
                        Value::Null,
                    );
                }
            }
        });
        if io.busy() && ready {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr(
                    ja,
                    "ダウンロードを準備しています…",
                    "Preparing download…",
                ));
            });
        }
        ui.separator();
        let mut action = OutputAction::None;
        let mut side = self.side;
        ui.horizontal(|ui| {
            ui.strong(tr(ja, "録音する面", "Recording side"));
            ui.selectable_value(&mut side, 0, tr(ja, "A面", "Side A"));
            ui.selectable_value(&mut side, 1, tr(ja, "B面", "Side B"));
        });
        if side != self.side {
            action = OutputAction::Side(side);
        }
        let owns = player.length > 0.;
        let active = owns && !player.finished();
        if !ready && self.error.is_some() {
            ui.colored_label(
                appearance::RED,
                tr(
                    ja,
                    "再生用音声の準備に失敗しました。生成済みのWAVは保存できます。",
                    "Playback preparation failed. You can still save the generated WAV files.",
                ),
            );
        } else if !ready {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(tr(
                    ja,
                    "再生用の音声を準備しています…",
                    "Preparing audio for playback…",
                ));
            });
        } else {
            ui.colored_label(
                appearance::GREEN,
                tr(
                    ja,
                    "デッキ録音の準備ができました",
                    "Ready for deck recording",
                ),
            );
        }
        ui.label(tr(
            ja,
            "デッキを録音待機にしてから再生を開始してください。",
            "Put the deck in recording standby before starting playback.",
        ));
        if let Some(seconds) = remaining {
            ui.colored_label(
                appearance::GREEN,
                if ja {
                    format!("{:.0}秒後に再生開始", seconds.ceil())
                } else {
                    format!("Starting in {:.0} seconds", seconds.ceil())
                },
            );
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    ready && !io.busy() && !active && remaining.is_none(),
                    appearance::primary(tr(ja, "5秒後に再生開始", "Start in 5 seconds")),
                )
                .clicked()
            {
                action = OutputAction::Countdown;
            }
            if ui
                .add_enabled(
                    active || (ready && !io.busy()),
                    appearance::primary(if active {
                        if player.paused() {
                            tr(ja, "再生を再開", "Resume")
                        } else {
                            tr(ja, "一時停止", "Pause")
                        }
                    } else {
                        tr(ja, "再生開始", "Start now")
                    }),
                )
                .clicked()
            {
                action = if active {
                    OutputAction::Toggle
                } else {
                    OutputAction::Start
                };
            }
            if ui
                .add_enabled(
                    owns || remaining.is_some(),
                    egui::Button::new(tr(ja, "停止", "Stop")),
                )
                .clicked()
            {
                action = OutputAction::Stop;
            }
        });
        ui.add_enabled_ui(owns, |ui| {
            ui.horizontal(|ui| {
                ui.small(crate::settings::time(if owns {
                    player.position()
                } else {
                    0.
                }));
                ui.spacing_mut().slider_width = 360.;
                let mut position = if owns { player.position() } else { 0. };
                if ui
                    .add(appearance::slider(
                        egui::Slider::new(&mut position, 0.0..=player.length.max(1.))
                            .show_value(false),
                    ))
                    .changed()
                {
                    action = OutputAction::Seek(position);
                }
                ui.small(crate::settings::time(if owns { player.length } else { 0. }));
            });
        });
        ui.horizontal(|ui| {
            ui.label(tr(ja, "音量", "Volume"));
            ui.spacing_mut().slider_width = 180.;
            let mut volume = player.volume;
            if ui
                .add(appearance::slider(
                    egui::Slider::new(&mut volume, 0.0..=1.).show_value(false),
                ))
                .changed()
            {
                action = OutputAction::Volume(volume);
            }
        });
        action
    }
}

impl Drop for Output {
    fn drop(&mut self) {
        self.stop();
    }
}
