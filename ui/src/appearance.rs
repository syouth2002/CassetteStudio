//! Shared visual language of the original Cassette Studio interface.
use crate::{
    model::*,
    settings::{time, tr},
};
use eframe::egui::{self, Color32};
pub const GREEN: Color32 = Color32::from_rgb(49, 94, 81);
pub const MUTED: Color32 = Color32::from_rgb(96, 110, 102);
pub const INK: Color32 = Color32::from_rgb(37, 51, 53);
pub const RED: Color32 = Color32::from_rgb(168, 62, 51);
pub const CONTROL_BORDER: Color32 = Color32::from_rgb(148, 164, 151);
pub const PAPER: Color32 = Color32::from_rgb(255, 254, 250);
pub const LINE: Color32 = Color32::from_rgb(227, 228, 220);
pub const SIDEBAR: Color32 = Color32::from_rgb(238, 239, 230);

pub fn install(ctx: &egui::Context) {
    ctx.set_theme(egui::Theme::Light);
    let mut style = (*ctx.style()).clone();
    let v = &mut style.visuals;
    *v = egui::Visuals::light();
    v.override_text_color = Some(INK);
    v.panel_fill = Color32::from_rgb(246, 244, 239);
    v.window_fill = PAPER;
    v.extreme_bg_color = PAPER;
    v.faint_bg_color = Color32::from_rgb(238, 240, 232);
    v.selection.bg_fill = Color32::from_rgb(220, 233, 224);
    v.selection.stroke = egui::Stroke::new(1.0_f32, GREEN);
    for widget in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
    ] {
        widget.bg_stroke = egui::Stroke::new(1.1_f32, CONTROL_BORDER);
        widget.corner_radius = egui::CornerRadius::same(6);
    }
    v.widgets.inactive.bg_fill = PAPER;
    v.widgets.inactive.weak_bg_fill = PAPER;
    v.widgets.hovered.bg_fill = Color32::from_rgb(224, 232, 220);
    v.widgets.hovered.weak_bg_fill = v.widgets.hovered.bg_fill;
    v.widgets.active.bg_fill = Color32::from_rgb(207, 224, 210);
    style.spacing.button_padding = egui::vec2(10., 6.);
    style.spacing.item_spacing = egui::vec2(8., 8.);
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(13.));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(12.));
    style.text_styles.insert(
        egui::TextStyle::Heading,
        egui::FontId::new(18., egui::FontFamily::Name("Title".into())),
    );
    ctx.set_style(style);
}

/// Slider-only visuals: keep buttons pale while making every rail visible.
pub fn slider(slider: egui::Slider<'_>) -> impl egui::Widget + '_ {
    move |ui: &mut egui::Ui| {
        ui.scope(|ui| {
            ui.spacing_mut().slider_rail_height = 6.;
            let v = ui.visuals_mut();
            v.widgets.inactive.bg_fill = CONTROL_BORDER;
            v.widgets.inactive.fg_stroke = egui::Stroke::new(2.0_f32, GREEN);
            v.widgets.hovered.fg_stroke = egui::Stroke::new(2.0_f32, GREEN);
            v.widgets.active.fg_stroke = egui::Stroke::new(2.0_f32, INK);
            v.selection.bg_fill = GREEN;
            ui.add(slider.trailing_fill(true))
        })
        .inner
    }
}

#[derive(Default)]
pub struct DialogActions {
    pub save: bool,
    pub close: bool,
}

/// Common dialog action order: Save then Close, anchored to the upper right.
pub fn dialog_header(
    ui: &mut egui::Ui,
    title: &str,
    save_enabled: Option<bool>,
    ja: bool,
) -> DialogActions {
    let mut actions = DialogActions::default();
    ui.horizontal(|ui| {
        ui.heading(title);
        let close = tr(ja, "閉じる", "Close");
        let save = tr(ja, "保存", "Save");
        if save_enabled.is_some() {
            right_space(ui, &[save, close]);
        } else {
            right_space(ui, &[close]);
        }
        if let Some(enabled) = save_enabled {
            actions.save = ui.add_enabled(enabled, primary(save)).clicked();
        }
        actions.close = ui.button(close).clicked();
    });
    actions
}

pub fn primary(label: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label).color(Color32::WHITE))
        .fill(GREEN)
        .stroke(egui::Stroke::new(1.0_f32, GREEN))
}
pub fn dark(label: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label).color(Color32::WHITE))
        .fill(INK)
        .stroke(egui::Stroke::new(1.0_f32, INK))
}
/// Paint the published four-point star independently of font glyph support.
pub fn automatic_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let response = ui.add(dark(&format!(
        "    {}",
        label.trim_start_matches('✦').trim_start()
    )));
    let center = egui::pos2(
        response.rect.left() + ui.spacing().button_padding.x + 5.,
        response.rect.center().y,
    );
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(center, Color32::WHITE);
    for i in 0..8 {
        let angle = std::f32::consts::FRAC_PI_4 * i as f32;
        let radius = if i % 2 == 0 { 6. } else { 2. };
        mesh.colored_vertex(
            center + egui::vec2(angle.cos(), angle.sin()) * radius,
            Color32::WHITE,
        );
    }
    for i in 0..8 {
        mesh.add_triangle(0, i + 1, (i + 1) % 8 + 1);
    }
    ui.painter().add(egui::Shape::mesh(mesh));
    response
}

pub fn danger(label: &str) -> egui::Button<'static> {
    egui::Button::new(egui::RichText::new(label).color(RED))
}

pub fn brand(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(34., 34.), egui::Sense::hover());
        ui.painter().rect_stroke(
            rect.shrink(1.),
            0.,
            egui::Stroke::new(2.0_f32, GREEN),
            egui::StrokeKind::Inside,
        );
        ui.painter().rect_filled(rect.shrink(6.), 0., GREEN);
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.;
            ui.label(
                egui::RichText::new("Cassette Studio")
                    .font(egui::FontId::new(
                        21.,
                        egui::FontFamily::Name("Title".into()),
                    ))
                    .strong()
                    .color(INK),
            );
            ui.label(
                egui::RichText::new("MAKE ROOM FOR YOUR MUSIC")
                    .size(12.)
                    .color(MUTED),
            );
        });
    });
}

pub fn header_cell<R>(
    ui: &mut egui::Ui,
    width: f32,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.allocate_ui_with_layout(
        egui::vec2(width, 24.),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.set_min_size(egui::vec2(width, 24.));
            add(ui)
        },
    )
}

pub fn column<R>(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    ui.allocate_ui_with_layout(size, egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.set_min_size(size);
        add(ui)
    })
}

/// Reserve the exact button widths, keeping their reading order left to right.
pub fn right_space(ui: &mut egui::Ui, labels: &[&str]) {
    let font = egui::TextStyle::Button.resolve(ui.style());
    let width: f32 = labels
        .iter()
        .map(|label| {
            ui.painter()
                .layout_no_wrap((*label).into(), font.clone(), GREEN)
                .size()
                .x
                + ui.spacing().button_padding.x * 2.
        })
        .sum::<f32>()
        + ui.spacing().item_spacing.x * labels.len().saturating_sub(1) as f32;
    ui.add_space((ui.available_size_before_wrap().x - width - ui.spacing().item_spacing.x).max(0.));
}

pub fn cassette(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(48., 32.), egui::Sense::hover());
    let painter = ui.painter();
    let ink = Color32::from_rgb(76, 73, 64);
    painter.rect(
        rect.shrink(1.),
        4.,
        Color32::from_rgb(223, 183, 129),
        egui::Stroke::new(2.0_f32, ink),
        egui::StrokeKind::Inside,
    );
    painter.rect_filled(
        egui::Rect::from_min_max(
            rect.min + egui::vec2(3., 24.),
            rect.max - egui::vec2(3., 3.),
        ),
        1.,
        Color32::from_rgb(198, 149, 98),
    );
    for x in [16., 32.] {
        painter.circle_filled(rect.min + egui::vec2(x, 15.), 3., ink);
    }
    painter.line_segment(
        [
            rect.min + egui::vec2(15., 27.),
            rect.min + egui::vec2(33., 27.),
        ],
        egui::Stroke::new(3.0_f32, Color32::from_rgb(116, 98, 74)),
    );
}

pub fn status(t: &Track, e: &Entry, s: &Settings, ja: bool) -> Vec<String> {
    let mut out = Vec::new();
    if e.continuation {
        out.push(tr(ja, "A面から引き継ぎ", "Continued from side A").into());
    }
    if e.fade_in > 0. {
        out.push(format!("IN {:.1}s", e.fade_in));
    }
    if e.fade_out > 0. {
        out.push(format!("OUT {:.1}s", e.fade_out));
    }
    if e.offset > 0. {
        out.push(format!("{}〜", time(e.offset)));
    }
    if t.audible() <= 0. {
        out.push(tr(ja, "全体が無音", "Entirely silent").into());
    } else if t.duration - t.audible() > 0.05 {
        out.push(tr(ja, "無音除去", "Silence trimmed").into());
    }
    if t.mono == Some(true) {
        out.push(tr(ja, "モノラル", "MONO").into());
    }
    if t.pseudo_stereo > 0 {
        out.push(format!(
            "{}：{}",
            tr(ja, "疑似ステレオ", "Pseudo stereo"),
            if ja {
                ["", "弱", "中", "強"][t.pseudo_stereo.min(3) as usize]
            } else {
                ["", "Weak", "Medium", "Strong"][t.pseudo_stereo.min(3) as usize]
            }
        ));
    }
    if s.remove_dc {
        out.push(
            tr(
                ja,
                if t.dc.is_some() {
                    "DC除去"
                } else {
                    "DC未解析"
                },
                if t.dc.is_some() {
                    "DC removal"
                } else {
                    "DC not analyzed"
                },
            )
            .into(),
        );
    }
    if s.normalize || s.remove_dc || t.manual_gain_db != 0. || t.pseudo_stereo > 0 {
        let stats = if t.pseudo_stereo > 0 {
            t.pseudo_analysis
                .as_ref()
                .filter(|a| a.strength == t.pseudo_stereo)
                .map(|a| {
                    if s.remove_dc {
                        &a.corrected_loudness
                    } else {
                        &a.loudness
                    }
                })
        } else if s.remove_dc {
            t.dc.as_ref().map(|d| &d.corrected_loudness)
        } else {
            t.loudness.as_ref()
        };
        if let Some(stats) = stats {
            let requested = if s.normalize {
                stats.integrated_lufs.map_or(0., |i| s.target_lufs - i)
            } else {
                0.
            } + t.manual_gain_db;
            let gain = requested.min(
                stats
                    .true_peak_db
                    .map_or(f64::INFINITY, |p| s.peak_db - 0.1 - p),
            );
            out.push(format!(
                "{gain:+.1} dB{}",
                if gain < requested - 0.01 {
                    tr(ja, " · ピーク制限", " · Peak limited")
                } else {
                    ""
                }
            ));
        } else {
            out.push(tr(ja, "音量未解析", "Loudness not analyzed").into());
        }
    }
    out
}

/// Raw measurements remain visible in All tracks even when correction is off.
pub fn row_status(t: &Track, e: &Entry, s: &Settings, ja: bool, source_view: bool) -> Vec<String> {
    let mut values = Vec::new();
    if source_view {
        if let Some(loudness) = &t.loudness {
            values.push(
                loudness
                    .integrated_lufs
                    .map(|v| format!("{} {v:.1} LUFS", tr(ja, "元音量", "Source")))
                    .unwrap_or_else(|| tr(ja, "音量測定不可", "LUFS unavailable").into()),
            );
            if let Some(peak) = loudness.true_peak_db {
                values.push(format!("TP {peak:.1} dBTP"));
            }
        } else {
            values.push(tr(ja, "音量未解析", "Loudness not analyzed").into());
        }
        if let Some(dc) = &t.dc {
            values.push(format!(
                "DC L {:+.4}% / R {:+.4}%",
                dc.offset[0] * 100.,
                dc.offset[1] * 100.
            ));
        } else {
            values.push(tr(ja, "DC未解析", "DC not analyzed").into());
        }
    }
    for label in status(t, e, s, ja) {
        if !values.contains(&label) {
            values.push(label);
        }
    }
    values
}

/// Keep numeric analysis in Properties / hover, leaving only actionable state.
pub fn compact_status(
    t: &Track,
    e: &Entry,
    s: &Settings,
    ja: bool,
    source_view: bool,
) -> Vec<String> {
    let mut labels = Vec::new();
    for label in status(t, e, s, ja) {
        if source_view
            && [
                "音量未解析",
                "DC未解析",
                "Loudness not analyzed",
                "DC not analyzed",
            ]
            .contains(&label.as_str())
        {
            continue;
        }
        let label = if source_view && label.contains("dB") {
            if label.contains("ピーク制限") || label.contains("Peak limited") {
                tr(ja, "ピーク制限", "Peak limited").into()
            } else {
                continue;
            }
        } else {
            label
        };
        if !labels.contains(&label) {
            labels.push(label);
        }
    }
    if source_view {
        if t.manual_gain_db != 0. {
            labels.push(tr(ja, "音量調整", "Gain adjusted").into());
        }
        if t.loudness.is_none() || t.dc.is_none() {
            let label = tr(ja, "未解析", "Not analyzed").to_owned();
            if !labels.contains(&label) {
                labels.push(label);
            }
        }
    }
    labels
}

pub fn badges(ui: &mut egui::Ui, values: &[String]) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4., 3.);
        let maximum = ui.max_rect().width().max(1.);
        for text in values {
            let galley =
                ui.painter()
                    .layout_no_wrap(text.clone(), egui::FontId::proportional(12.), GREEN);
            let size = egui::vec2(
                (galley.size().x + 10.).min(maximum),
                galley.size().y.max(16.) + 4.,
            );
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
            ui.painter().rect(
                rect,
                3.,
                Color32::from_rgb(223, 232, 220),
                egui::Stroke::new(1.0_f32, Color32::from_rgb(173, 192, 177)),
                egui::StrokeKind::Inside,
            );
            ui.painter()
                .with_clip_rect(rect.shrink2(egui::vec2(5., 2.)))
                .galley(
                    egui::pos2(rect.left() + 5., rect.center().y - galley.size().y / 2.),
                    galley,
                    GREEN,
                );
            response.on_hover_text(text);
        }
    });
}

/// Shared large single-line input for URLs and collection names.
pub fn wide_text_input(
    ui: &mut egui::Ui,
    text: &mut String,
    id: &str,
    hint: &str,
) -> egui::Response {
    ui.add_sized(
        [ui.available_width(), 44.],
        egui::TextEdit::singleline(text)
            .id_salt(id)
            .desired_width(f32::INFINITY)
            .font(egui::FontId::proportional(16.))
            .margin(egui::Margin::symmetric(10, 12))
            .hint_text(hint),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn library_measurements_are_visible_when_processing_is_disabled() {
        let mut track:Track=serde_json::from_value(serde_json::json!({"id":"t","title":"Title","artist":"Artist","filename":"audio.wav","source":"local","duration":10.,"trim_start":0.,"trim_end":10.,"loudness":{"integrated_lufs":-22.1,"true_peak_db":-0.8},"dc":{"offset":[0.001,-0.002],"corrected_loudness":{"integrated_lufs":-22.1,"true_peak_db":-0.8}}})).unwrap();
        let entry = crate::arrangement::whole(&track);
        let settings = Settings::default();
        let labels = row_status(&track, &entry, &settings, true, true).join(" / ");
        for expected in [
            "元音量 -22.1 LUFS",
            "TP -0.8 dBTP",
            "DC L +0.1000% / R -0.2000%",
        ] {
            assert!(labels.contains(expected), "{labels}");
        }
        assert!(row_status(&track, &entry, &settings, true, false).is_empty());
        track.loudness = None;
        track.dc = None;
        assert_eq!(
            row_status(&track, &entry, &settings, true, true),
            ["音量未解析", "DC未解析"]
        );
    }
    #[test]
    fn processing_status_covers_trim_continuation_and_peak_protection() {
        let track:Track=serde_json::from_value(serde_json::json!({
            "id":"test","title":"Test","artist":"Artist","filename":"test.wav","source":"local",
            "duration":100.,"trim_start":1.,"trim_end":99.,"mono":true,"manual_gain_db":6.,
            "loudness":{"integrated_lufs":-24.,"true_peak_db":-3.},
            "dc":{"offset":[0.01,0.01],"corrected_loudness":{"integrated_lufs":-24.,"true_peak_db":-3.}}
        })).unwrap();
        let entry = Entry {
            track_id: track.id.clone(),
            offset: 10.,
            duration: 40.,
            fade_in: 2.,
            fade_out: 3.,
            continuation: true,
        };
        let settings = Settings {
            normalize: true,
            remove_dc: true,
            ..Default::default()
        };
        let text = status(&track, &entry, &settings, true).join(" / ");
        for label in [
            "A面から引き継ぎ",
            "IN 2.0s",
            "OUT 3.0s",
            "0:10〜",
            "無音除去",
            "モノラル",
            "DC除去",
            "+1.9 dB",
            "ピーク制限",
        ] {
            assert!(text.contains(label), "Missing {label}: {text}");
        }
        let mut stereo = track;
        stereo.pseudo_stereo = 2;
        let english = status(&stereo, &entry, &settings, false).join(" / ");
        assert!(english.contains("Pseudo stereo：Medium"));
        assert!(english.contains("Loudness not analyzed"));
    }
}
