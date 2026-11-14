//! Visible long-operation progress and completion/error notifications.
use crate::{
    appearance,
    settings::{time, tr},
};
use eframe::egui::{self, Color32, RichText};
use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Success,
    Warning,
    Error,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ImportPhase {
    Reading,
    Analyzing,
    Applying,
}
pub struct ImportProgress {
    pub url: Option<String>,
    pub names: Vec<String>,
    pub phase: ImportPhase,
    pub started: f64,
    pub read_errors: Vec<String>,
}
struct Notice {
    kind: NoticeKind,
    message: String,
    shown_at: Option<f64>,
}
#[derive(Default)]
pub struct Feedback {
    pub import: Option<ImportProgress>,
    notice: Option<Notice>,
    queue: VecDeque<Notice>,
}
impl Feedback {
    pub fn begin_import(&mut self, names: Vec<String>, now: f64) {
        self.import = Some(ImportProgress {
            url: None,
            names,
            phase: ImportPhase::Reading,
            started: now,
            read_errors: vec![],
        });
    }
    pub fn begin_url_import(&mut self, url: String, now: f64) {
        self.import = Some(ImportProgress {
            url: Some(url),
            names: vec![],
            phase: ImportPhase::Analyzing,
            started: now,
            read_errors: vec![],
        });
    }
    pub fn importing(&self) -> bool {
        self.import.is_some()
    }
    pub fn blocking(&self) -> bool {
        self.importing()
            || self
                .notice
                .as_ref()
                .is_some_and(|n| n.kind != NoticeKind::Info)
    }
    pub fn notify(&mut self, kind: NoticeKind, message: String) {
        if self
            .notice
            .as_ref()
            .is_some_and(|n| n.kind == kind && n.message == message)
            || self
                .queue
                .iter()
                .any(|n| n.kind == kind && n.message == message)
        {
            return;
        }
        let notice = Notice {
            kind,
            message,
            shown_at: None,
        };
        if self
            .notice
            .as_ref()
            .is_none_or(|n| n.kind == NoticeKind::Info)
        {
            self.notice = Some(notice);
        } else if kind != NoticeKind::Info {
            self.queue.push_back(notice);
        }
    }
    fn dismiss(&mut self) {
        self.notice = self.queue.pop_front();
    }
    pub fn ui(&mut self, ctx: &egui::Context, ja: bool, show_info: bool) {
        let width = (ctx.content_rect().width() - 64.).clamp(220., 680.);
        if let Some(progress) = &self.import {
            egui::Modal::new(egui::Id::new("import-progress")).show(ctx, |ui| {
                ui.set_width(width);
                ui.add_space(10.);
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(34.).color(appearance::GREEN));
                    ui.heading(if progress.url.is_some() {
                        tr(ja, "URLから取り込んでいます", "Importing from URL")
                    } else {
                        tr(ja, "ファイルを追加しています", "Adding media files")
                    });
                });
                ui.separator();
                ui.label(
                    RichText::new(match progress.phase {
                        ImportPhase::Reading => tr(ja, "ファイルを読み込み中…", "Reading files…"),
                        ImportPhase::Analyzing if progress.url.is_some() => tr(
                            ja,
                            "URLの音源を取得・解析中…",
                            "Downloading and analyzing audio…",
                        ),
                        ImportPhase::Analyzing => tr(
                            ja,
                            "音源を取り込み・解析中…",
                            "Importing and analyzing audio…",
                        ),
                        ImportPhase::Applying => {
                            tr(ja, "曲リストに反映中…", "Updating the track list…")
                        }
                    })
                    .size(17.)
                    .color(appearance::GREEN),
                );
                ui.label(tr(
                    ja,
                    "完了すると曲リストへ追加されます。処理中はこのままお待ちください。",
                    "The tracks will appear in the list when processing finishes. Please wait.",
                ));
                ui.add_space(6.);
                if let Some(url) = &progress.url {
                    ui.label(tr(ja, "取り込み元のURL", "Source URL"));
                    ui.add(egui::Label::new(url).truncate()).on_hover_text(url);
                } else {
                    ui.label(format!(
                        "{} {}",
                        progress.names.len(),
                        tr(ja, "ファイル", "files")
                    ));
                    for name in progress.names.iter().take(3) {
                        ui.add(egui::Label::new(name).truncate());
                    }
                    if progress.names.len() > 3 {
                        ui.small(format!(
                            "{} {}",
                            tr(ja, "ほか", "and"),
                            progress.names.len() - 3
                        ));
                    }
                }
                ui.separator();
                ui.small(format!(
                    "{} {}",
                    tr(ja, "経過時間", "Elapsed"),
                    time((ctx.input(|i| i.time) - progress.started).max(0.))
                ));
                ui.add_space(8.);
            });
            return;
        }
        let Some(notice) = &mut self.notice else {
            return;
        };
        if notice.kind == NoticeKind::Info {
            if !show_info {
                return;
            }
            let shown = *notice.shown_at.get_or_insert(ctx.input(|i| i.time));
            if ctx.input(|i| i.time) - shown >= 8. {
                self.dismiss();
                return;
            }
            let mut close = false;
            egui::Area::new(egui::Id::new("completion-banner"))
                .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0., -126.))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .fill(Color32::from_rgb(225, 237, 223))
                        .stroke(egui::Stroke::new(1.5_f32, appearance::GREEN))
                        .inner_margin(16.)
                        .show(ui, |ui| {
                            ui.set_max_width(width);
                            ui.horizontal(|ui| {
                                notice_icon(ui, NoticeKind::Success);
                                ui.label(RichText::new(&notice.message).size(15.));
                                if ui.button(tr(ja, "閉じる", "Close")).clicked() {
                                    close = true;
                                }
                            });
                        });
                });
            if close {
                self.dismiss();
            }
            return;
        }
        let mut close = false;
        let modal = egui::Modal::new(egui::Id::new("operation-result")).show(ctx, |ui| {
            ui.set_width(width);
            ui.horizontal(|ui| {
                notice_icon(ui, notice.kind);
                ui.heading(
                    RichText::new(match notice.kind {
                        NoticeKind::Success => tr(ja, "処理が完了しました", "Operation completed"),
                        NoticeKind::Warning => {
                            tr(ja, "処理結果を確認してください", "Please review the result")
                        }
                        NoticeKind::Error => tr(ja, "エラーが発生しました", "An error occurred"),
                        NoticeKind::Info => tr(ja, "お知らせ", "Notice"),
                    })
                    .color(notice_color(notice.kind)),
                );
                appearance::right_space(ui, &[tr(ja, "閉じる", "Close")]);
                if ui.button(tr(ja, "閉じる", "Close")).clicked() {
                    close = true;
                }
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .max_height((ctx.content_rect().height() - 260.).clamp(100., 560.))
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(RichText::new(&notice.message).size(15.))
                            .wrap()
                            .selectable(true),
                    );
                });
            ui.separator();
        });
        if close || (modal.is_top_modal && ctx.input(|i| i.key_pressed(egui::Key::Escape))) {
            self.dismiss();
        }
    }
}
fn notice_color(kind: NoticeKind) -> Color32 {
    match kind {
        NoticeKind::Error => appearance::RED,
        NoticeKind::Warning => Color32::from_rgb(155, 100, 27),
        _ => appearance::GREEN,
    }
}
fn notice_icon(ui: &mut egui::Ui, kind: NoticeKind) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(30., 30.), egui::Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    p.circle_filled(c, 14., notice_color(kind));
    let stroke = egui::Stroke::new(2.3_f32, Color32::WHITE);
    match kind {
        NoticeKind::Error => {
            p.line_segment([c - egui::vec2(5., 5.), c + egui::vec2(5., 5.)], stroke);
            p.line_segment([c + egui::vec2(-5., 5.), c + egui::vec2(5., -5.)], stroke);
        }
        NoticeKind::Warning => {
            p.line_segment([c - egui::vec2(0., 7.), c + egui::vec2(0., 2.)], stroke);
            p.circle_filled(c + egui::vec2(0., 7.), 1.5, Color32::WHITE);
        }
        _ => {
            p.line_segment([c + egui::vec2(-7., 0.), c + egui::vec2(-2., 5.)], stroke);
            p.line_segment([c + egui::vec2(-2., 5.), c + egui::vec2(7., -5.)], stroke);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn routine_notifications_cannot_replace_an_error() {
        let mut feedback = Feedback::default();
        feedback.notify(NoticeKind::Info, "Completed".into());
        feedback.notify(NoticeKind::Error, "Import failed".into());
        feedback.notify(NoticeKind::Info, "Completed".into());
        assert!(feedback.blocking());
        assert_eq!(feedback.notice.as_ref().unwrap().message, "Import failed");
        feedback.notify(NoticeKind::Success, "Saved".into());
        feedback.dismiss();
        assert_eq!(feedback.notice.as_ref().unwrap().message, "Saved");
    }
}
