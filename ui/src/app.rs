use crate::{
    appearance,
    arrangement::{self, fit, whole},
    feedback::{Feedback, ImportPhase, NoticeKind},
    graph::Graph,
    jcard::Card,
    model::*,
    platform,
    playback::Player,
    settings::{self, time, tr},
    transport::{Event, Tag, Transport},
};
use eframe::egui::{self, Color32, RichText};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};

#[path = "app_dialogs.rs"]
mod app_dialogs;
#[path = "app_events.rs"]
mod app_events;
#[path = "app_layout.rs"]
mod app_layout;
#[path = "app_playback.rs"]
mod app_playback;
#[path = "app_tracks.rs"]
mod app_tracks;

enum Dialog {
    Properties(Box<crate::properties::Properties>),
    Options,
    Tone {
        frequency: f64,
        db: f64,
        seconds: f64,
    },
    Url(String),
    Settings {
        target: Option<String>,
        value: Settings,
    },
    Name {
        kind: String,
        name: String,
        optimize: bool,
    },
    Rename(String),
    Add(BTreeSet<String>),
    Gain {
        track: Box<Track>,
        gain: f64,
        strength: u8,
    },
    Entry {
        index: usize,
        value: Entry,
    },
    Batch(BTreeSet<String>),
    Delete {
        collection: bool,
    },
    Output(Box<crate::output::Output>),
    PlaybackPreparing {
        generation: u64,
        name: String,
        loading: bool,
    },
}
impl Dialog {
    fn title(&self, ja: bool) -> &'static str {
        match self {
            Self::PlaybackPreparing { .. } => tr(ja, "再生データを準備", "Preparing playback"),
            Self::Properties(_) => tr(ja, "プロパティ", "Properties"),
            Self::Options => tr(ja, "オプション", "Options"),
            Self::Tone { .. } => tr(ja, "基準音", "Reference tone"),
            Self::Url(_) => tr(ja, "URL追加", "Add URL"),
            Self::Settings {
                target: Some(_), ..
            } => tr(
                ja,
                "カセットと録音の設定",
                "Cassette and recording settings",
            ),
            Self::Settings { .. } => tr(ja, "作成・取り込み設定", "Creation / import settings"),
            Self::Name { kind, .. } if kind == "playlist" => {
                tr(ja, "プレイリストを作成", "Create playlist")
            }
            Self::Name { kind, .. } if kind == "blank" => {
                tr(ja, "空カセットを作成", "Create blank cassette")
            }
            Self::Name { optimize: true, .. } => {
                tr(ja, "自動でカセット作成", "Create cassette automatically")
            }
            Self::Name { .. } => tr(
                ja,
                "選曲／曲順でカセット作成",
                "Create cassette from selection / order",
            ),
            Self::Rename(_) => tr(ja, "名前変更", "Rename"),
            Self::Add(_) => tr(ja, "ライブラリから追加", "Add from library"),
            Self::Gain { track, .. } if track.mono == Some(true) => {
                tr(ja, "音量調整・疑似ステレオ", "Loudness / pseudo stereo")
            }
            Self::Gain { .. } => tr(ja, "曲の音量を調整", "Adjust track loudness"),
            Self::Entry { .. } => tr(ja, "再生範囲・フェード", "Playback range / fades"),
            Self::Batch(_) => tr(ja, "自動カセット作成", "Create cassettes automatically"),
            Self::Delete { .. } => tr(ja, "削除の確認", "Confirm deletion"),
            Self::Output(_) => tr(ja, "WAV出力・デッキ録音", "WAV export / deck recording"),
        }
    }
}
pub struct Studio {
    io: Transport,
    library: Library,
    active: Option<String>,
    side: usize,
    selected: BTreeSet<usize>,
    defaults: Settings,
    language: String,
    ja: bool,
    dialog: Option<Dialog>,
    feedback: Feedback,
    player: Player,
    preview: crate::playback::PreviewControl,
    graph: Graph,
    card: Option<Card>,
    card_clipboard: Option<crate::jcard::Clipboard>,
    exports: HashMap<String, (Value, [String; 2])>,
    session: crate::play_session::Session,
    exit_requested: bool,
    mono_attempted: BTreeSet<String>,
    prepared_audio: Option<(String, usize, Value, Vec<u8>)>,
}
fn value<T: serde::Serialize>(v: &T) -> Value {
    serde_json::to_value(v).expect("serializable model")
}
fn render_key(c: &Collection, tracks: &[Track]) -> Value {
    let ids = c
        .sides
        .iter()
        .flatten()
        .map(|e| e.track_id.as_str())
        .collect::<BTreeSet<_>>();
    let sources = tracks
        .iter()
        .filter(|t| ids.contains(t.id.as_str()))
        .map(|t| {
            json!([
                t.id,
                t.manual_gain_db,
                t.pseudo_stereo,
                t.trim_start,
                t.trim_end,
                t.content_hash,
                t.loudness,
                t.dc,
                t.pseudo_analysis
            ])
        })
        .collect::<Vec<_>>();
    json!({"sides":c.sides,"settings":c.settings,"sources":sources})
}
pub fn numbered(base: &str, names: impl Iterator<Item = String>) -> String {
    let names: BTreeSet<_> = names.collect();
    if !names.contains(base) {
        return base.into();
    }
    for n in 2.. {
        let name = format!("{base} ({n})");
        if !names.contains(&name) {
            return name;
        }
    }
    unreachable!()
}
impl Studio {
    pub fn new(cc: &eframe::CreationContext<'_>, base: String) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "NotoSansJP".into(),
            egui::FontData::from_static(include_bytes!("../../assets/fonts/NotoSansJP.ttf")).into(),
        );
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .push("NotoSansJP".into());
        }
        fonts.font_data.insert(
            "NotoSansJP-Semibold".into(),
            egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/NotoSansJP-Semibold.ttf"
            ))
            .into(),
        );
        let mut heading_fonts = fonts.families[&egui::FontFamily::Proportional].clone();
        heading_fonts.insert(0, "NotoSansJP-Semibold".into());
        fonts
            .families
            .insert(egui::FontFamily::Name("Title".into()), heading_fonts);
        cc.egui_ctx.set_fonts(fonts);
        appearance::install(&cc.egui_ctx);
        let language = cc
            .storage
            .and_then(|s| s.get_string("language"))
            .unwrap_or_else(|| "auto".into());
        let defaults = cc
            .storage
            .and_then(|s| s.get_string("settings"))
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        let ja = language == "ja"
            || (language == "auto" && platform::locale().to_lowercase().starts_with("ja"));
        let mut app = Self {
            io: Transport::new(base),
            library: Library::default(),
            active: None,
            side: 0,
            selected: Default::default(),
            defaults,
            language,
            ja,
            dialog: None,
            feedback: Feedback::default(),
            player: Player::new(),
            preview: Default::default(),
            graph: Graph::default(),
            card: None,
            card_clipboard: None,
            exports: Default::default(),
            session: Default::default(),
            exit_requested: false,
            mono_attempted: BTreeSet::new(),
            prepared_audio: None,
        };
        app.reload();
        app
    }
    fn reload(&mut self) {
        self.io
            .json(Tag::Library, "GET", "/api/library", Value::Null);
    }
    fn current(&self) -> Option<&Collection> {
        self.active
            .as_ref()
            .and_then(|id| self.library.collections.iter().find(|c| &c.id == id))
    }
    fn config(&self) -> Settings {
        self.current()
            .map(|c| c.settings.clone())
            .unwrap_or_else(|| self.defaults.clone())
    }
    fn is_tape(&self) -> bool {
        self.current().is_some_and(|c| c.kind == "cassette")
    }
    fn entries(&self) -> Vec<Entry> {
        match self.current() {
            Some(c) if c.kind == "cassette" => c.sides[self.side].clone(),
            Some(c) => c
                .tracks
                .iter()
                .filter_map(|id| self.library.tracks.iter().find(|t| &t.id == id))
                .map(whole)
                .collect(),
            None => self.library.tracks.iter().map(whole).collect(),
        }
    }
    fn ids(&self) -> Vec<String> {
        let es = self.entries();
        es.iter()
            .enumerate()
            .filter(|(i, _)| self.selected.is_empty() || self.selected.contains(i))
            .map(|(_, e)| e.track_id.clone())
            .collect()
    }
    fn save_collection(&mut self, c: Collection) {
        self.save_collection_notice(c, None);
    }
    fn save_collection_notice(&mut self, c: Collection, notice: Option<String>) {
        self.save_collection_request(c, Tag::Changed(notice));
    }
    fn save_collection_request(&mut self, c: Collection, tag: Tag) {
        if !c.id.is_empty()
            && let Some(old) = self
                .library
                .collections
                .iter_mut()
                .find(|old| old.id == c.id)
        {
            *old = c.clone();
        }
        self.io.json(tag, "POST", "/api/collections", value(&c));
    }
    fn select(&mut self, id: Option<String>) {
        self.active = id;
        self.selected.clear();
        self.side = 0;
    }
    fn notify_kind(&mut self, s: String, kind: NoticeKind) {
        let message = if !self.ja {
            let en: HashMap<String, String> =
                serde_json::from_str(include_str!("../../web/en.json")).unwrap_or_default();
            en.get(&s).cloned().unwrap_or(s)
        } else {
            s
        };
        self.feedback.notify(kind, message);
    }
    fn notify(&mut self, s: String) {
        self.notify_kind(s, NoticeKind::Info);
    }
    fn notify_success(&mut self, s: String) {
        self.notify_kind(s, NoticeKind::Success);
    }
    fn notify_warning(&mut self, s: String) {
        self.notify_kind(s, NoticeKind::Warning);
    }
    fn notify_error(&mut self, s: String) {
        self.notify_kind(s, NoticeKind::Error);
    }

    fn entries_len(&self) -> usize {
        self.entries().len()
    }

    fn append(&mut self, ids: Vec<String>) {
        if let Some(id) = self.active.clone() {
            self.append_to(&id, self.side, ids, None, None);
        }
    }
    fn append_to(
        &mut self,
        id: &str,
        side: usize,
        ids: Vec<String>,
        notice: Option<String>,
        import_warning: Option<bool>,
    ) {
        let requested = ids.len();
        let Some(mut c) = self
            .library
            .collections
            .iter()
            .find(|c| c.id == id)
            .cloned()
        else {
            if import_warning.is_some() {
                self.feedback.import = None;
                self.notify_warning(
                    tr(
                        self.ja,
                        "追加先が見つかりません。取り込んだ音源はライブラリに保存しました。",
                        "The destination is unavailable. Imported audio was saved to the library.",
                    )
                    .into(),
                );
            }
            return;
        };
        let added = arrangement::append(&mut c, &self.library.tracks, &ids, side);
        let skipped = requested - added;
        let notice = if skipped > 0 {
            Some(format!(
                "{}{} {}",
                notice.map(|s| s + "\n").unwrap_or_default(),
                skipped,
                tr(
                    self.ja,
                    "曲は空き時間がないためライブラリに保持しました。",
                    "tracks remain in the library because the tape is full."
                )
            ))
        } else {
            notice
        };
        if added > 0 {
            if let Some(warning) = import_warning {
                if let Some(progress) = &mut self.feedback.import {
                    progress.phase = ImportPhase::Applying;
                }
                self.save_collection_request(
                    c,
                    Tag::ImportSaved {
                        message: notice.unwrap_or_else(|| {
                            tr(self.ja, "曲を追加しました", "Tracks added").into()
                        }),
                        warning: warning || skipped > 0,
                    },
                );
            } else {
                self.save_collection_notice(c, notice);
            }
        } else if let Some(message) = notice {
            if import_warning.is_some() {
                self.feedback.import = None;
            }
            self.notify_kind(
                message,
                if import_warning == Some(true) || skipped > 0 {
                    NoticeKind::Warning
                } else if import_warning.is_some() {
                    NoticeKind::Success
                } else {
                    NoticeKind::Info
                },
            );
        }
    }
    fn move_entry(&mut self, index: usize, delta: isize) {
        let Some(mut c) = self.current().cloned() else {
            return;
        };
        let n = index.checked_add_signed(delta);
        if let Some(n) = n {
            if c.kind == "cassette" && n < c.sides[self.side].len() {
                c.sides[self.side].swap(index, n);
            } else if c.kind == "playlist" && n < c.tracks.len() {
                c.tracks.swap(index, n);
            } else {
                return;
            }
            self.selected.clear();
            self.selected.insert(n);
            self.save_collection_request(c, Tag::Reordered);
        }
    }
    fn move_opposite(&mut self) {
        let Some(mut c) = self.current().cloned() else {
            return;
        };
        if c.kind != "cassette" {
            return;
        }
        let other = 1 - self.side;
        let mut target = c.sides[other].clone();
        for (i, e) in c.sides[self.side].iter().enumerate() {
            if !self.selected.contains(&i) {
                continue;
            }
            let mut moved = e.clone();
            let used = c.settings.lead_in
                + side_duration(&target, c.settings.gap)
                + if target.is_empty() {
                    0.0
                } else {
                    c.settings.gap
                };
            if !fit(
                &mut moved,
                c.settings.side_seconds - used,
                e.offset + e.duration,
                c.settings.fade,
            ) {
                self.notify_warning(
                    tr(
                        self.ja,
                        "反対面に空きがありません",
                        "The opposite side has no available space",
                    )
                    .into(),
                );
                return;
            }
            target.push(moved);
        }
        c.sides[other] = target;
        let mut index = 0;
        c.sides[self.side].retain(|_| {
            let keep = !self.selected.contains(&index);
            index += 1;
            keep
        });
        self.selected.clear();
        self.save_collection(c);
    }
    fn remove_selection(&mut self) {
        if let Some(mut c) = self.current().cloned() {
            if c.kind == "cassette" {
                let mut i = 0;
                c.sides[self.side].retain(|_| {
                    let keep = !self.selected.contains(&i);
                    i += 1;
                    keep
                });
            } else {
                let mut i = 0;
                c.tracks.retain(|_| {
                    let keep = !self.selected.contains(&i);
                    i += 1;
                    keep
                });
            }
            self.save_collection(c);
        } else {
            self.io.json(
                Tag::Changed(None),
                "DELETE",
                "/api/tracks",
                json!({"tracks":self.ids()}),
            );
        }
        self.selected.clear();
    }
    fn make_name(&mut self, kind: &str, optimize: bool) {
        let name = numbered(
            if kind == "playlist" {
                "Playlist"
            } else {
                "My Mixtape"
            },
            self.library.collections.iter().map(|c| c.name.clone()),
        );
        self.dialog = Some(Dialog::Name {
            kind: kind.into(),
            name,
            optimize,
        });
    }
}
impl eframe::App for Studio {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.card.is_none() {
            platform::editor_viewport(None, ctx.pixels_per_point());
        }
        self.events(ctx);
        if ctx.input(|i| i.viewport().close_requested()) && !self.exit_requested {
            self.exit_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if let Some(c) = &mut self.card {
                c.request_close();
            }
        }
        let now = ctx.input(|i| i.time);
        self.playback_tick(now);
        if let Some(mut card) = self.card.take() {
            let close = card.ui(ctx, self.ja, &self.library, &mut self.io);
            if !close {
                self.card = Some(card);
            } else {
                platform::editor_viewport(None, ctx.pixels_per_point());
                self.card_clipboard = card.clipboard.take();
            }
        } else {
            self.top(ctx);
            self.player_ui(ctx);
            self.library_panel(ctx);
            self.tracks(ctx);
            if !self.feedback.blocking() {
                self.dialog_ui(ctx);
            }
            if let Some((id, side)) = self.graph.ui(ctx, self.ja) {
                self.side_graph_for(id, side);
            }
        }
        if !self.io.busy() && !self.feedback.importing() {
            let dropped = ctx.input(|i| i.raw.dropped_files.clone());
            if !dropped.is_empty() {
                platform::read_dropped(dropped, self.io.tx.clone(), ctx.clone());
            }
        }
        self.feedback.ui(
            ctx,
            self.ja,
            self.dialog.is_none() && self.card.is_none() && !self.graph.show,
        );
        self.io.tick(ctx);
        if self.exit_requested && self.card.is_none() && !self.io.busy() {
            self.stop();
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
    fn auto_save_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(1)
    }
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        storage.set_string("language", self.language.clone());
        storage.set_string("settings", serde_json::to_string(&self.defaults).unwrap());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn render_identity_ignores_names_but_tracks_audio_and_arrangement_changes() {
        let t:Track=serde_json::from_value(json!({"id":"track","title":"Title","artist":"Artist","filename":"a.wav","source":"local","duration":8.,"trim_start":0.,"trim_end":8.})).unwrap();
        let mut tracks = vec![t.clone()];
        let mut c = Collection {
            id: "tape".into(),
            name: "Tape".into(),
            kind: "cassette".into(),
            tracks: vec![t.id.clone()],
            sides: [vec![whole(&t)], vec![]],
            settings: Default::default(),
        };
        let original = render_key(&c, &tracks);
        c.name = "Renamed".into();
        tracks[0].title = "New title".into();
        assert_eq!(render_key(&c, &tracks), original);
        c.settings.gap += 0.5;
        assert_ne!(render_key(&c, &tracks), original);
        c.settings.gap -= 0.5;
        c.sides[0][0].fade_out = 1.;
        assert_ne!(render_key(&c, &tracks), original);
        c.sides[0][0].fade_out = 0.;
        tracks[0].manual_gain_db = 1.;
        assert_ne!(render_key(&c, &tracks), original);
        tracks[0].manual_gain_db = 0.;
        tracks[0].dc = Some(DcAnalysis {
            offset: [0.1, 0.],
            corrected_loudness: Loudness {
                integrated_lufs: Some(-20.),
                true_peak_db: Some(-1.),
            },
        });
        assert_ne!(render_key(&c, &tracks), original);
    }
    #[test]
    fn manual_fit_and_names() {
        let mut e = Entry {
            track_id: "x".into(),
            offset: 0.,
            duration: 90.,
            fade_in: 0.,
            fade_out: 5.,
            continuation: false,
        };
        assert!(fit(&mut e, 80.15, 90., 5.));
        assert_eq!(e.duration, 80.1);
        assert_eq!(e.fade_out, 5.);
        assert!(fit(&mut e, 100., 90., 5.));
        assert_eq!(e.fade_out, 0.);
        assert_eq!(
            numbered("Mix", ["Mix".into(), "Mix (2)".into()].into_iter()),
            "Mix (3)"
        );
    }
}
