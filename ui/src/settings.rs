use crate::model::Settings;
use eframe::egui::{self, Ui};
pub fn tr(ja: bool, jp: &'static str, en: &'static str) -> &'static str {
    if ja { jp } else { en }
}
pub fn time(s: f64) -> String {
    let s = s.max(0.0).round() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}
fn number(ui: &mut Ui, label: &str, v: &mut f64, range: std::ops::RangeInclusive<f64>, step: f64) {
    ui.label(label);
    let changed = ui
        .add(
            egui::DragValue::new(&mut *v)
                .range(range)
                .speed(step)
                .fixed_decimals(1)
                .custom_parser(move |s| s.parse::<f64>().ok().map(|v| (v / step).round() * step)),
        )
        .changed();
    if changed {
        *v = (*v / step).round() * step;
    }
}
pub fn fields(ui: &mut Ui, s: &mut Settings, ja: bool) {
    egui::Grid::new("recording-fields")
        .num_columns(4)
        .spacing([12.0, 10.0])
        .show(ui, |ui| {
            ui.label(tr(ja, "テープ時間", "Tape length"));
            egui::ComboBox::from_id_salt("preset")
                .selected_text(tr(ja, "選択…", "Choose…"))
                .show_ui(ui, |ui| {
                    for minutes in [46, 60, 74, 80, 90, 120] {
                        if ui.button(format!("{minutes} min")).clicked() {
                            s.side_seconds = minutes as f64 * 30.0;
                        }
                    }
                });
            ui.label(tr(ja, "片面の長さ（分／秒）", "Side length (min / sec)"));
            ui.horizontal(|ui| {
                let mut min = (s.side_seconds / 60.).floor();
                let mut sec = s.side_seconds % 60.;
                let a = ui
                    .add(egui::DragValue::new(&mut min).range(0.0..=120.).speed(1.))
                    .changed();
                ui.label(":");
                let b = ui
                    .add(egui::DragValue::new(&mut sec).range(0.0..=59.5).speed(0.5))
                    .changed();
                if a || b {
                    s.side_seconds = (min * 60. + sec).clamp(1., 7200.);
                }
            });
            ui.end_row();
            number(
                ui,
                tr(ja, "曲間の無音（秒）", "Track gap (s)"),
                &mut s.gap,
                0.0..=60.,
                0.5,
            );
            number(
                ui,
                tr(ja, "A/B面先頭の無音（秒）", "A/B leading silence (s)"),
                &mut s.lead_in,
                0.0..=s.side_seconds,
                0.5,
            );
            ui.end_row();
            number(
                ui,
                tr(ja, "末尾を埋める空き時間（秒）", "Fill threshold (s)"),
                &mut s.fill_threshold,
                0.0..=7200.,
                0.5,
            );
            number(
                ui,
                tr(ja, "フェード（秒）", "Fade (s)"),
                &mut s.fade,
                0.5..=60.,
                0.5,
            );
            ui.end_row();
            ui.label(tr(ja, "A→B面の引き継ぎ", "A→B continuation"));
            egui::ComboBox::from_id_salt("continuation")
                .selected_text(if s.continuation == "restart" {
                    tr(ja, "最初から", "Restart")
                } else {
                    tr(ja, "途中からフェードイン", "Resume with fade-in")
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut s.continuation,
                        "restart".into(),
                        tr(ja, "最初から", "Restart"),
                    );
                    ui.selectable_value(
                        &mut s.continuation,
                        "resume".into(),
                        tr(ja, "途中からフェードイン", "Resume with fade-in"),
                    );
                });
            ui.label(tr(ja, "WAV形式（DAT向け）", "WAV format (for DAT)"));
            egui::ComboBox::from_id_salt("sample-rate")
                .selected_text(match s.wav_sample_rate {
                    Some(48000) => "16bit / 48kHz",
                    Some(44100) => "16bit / 44.1kHz",
                    _ => tr(ja, "未設定", "Default"),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut s.wav_sample_rate, None, tr(ja, "未設定", "Default"));
                    ui.selectable_value(&mut s.wav_sample_rate, Some(48000), "16bit / 48kHz");
                    ui.selectable_value(&mut s.wav_sample_rate, Some(44100), "16bit / 44.1kHz");
                });
            ui.end_row();
            number(
                ui,
                tr(ja, "無音とみなす音量（dB）", "Silence threshold (dB)"),
                &mut s.silence_db,
                -100.0..=-10.,
                1.,
            );
            number(
                ui,
                tr(ja, "無音の最小継続時間（秒）", "Minimum silence (s)"),
                &mut s.silence_min,
                0.1..=10.,
                0.1,
            );
            ui.end_row();
            ui.checkbox(&mut s.normalize, tr(ja, "音量統一", "Normalize loudness"));
            ui.label("");
            ui.checkbox(
                &mut s.remove_dc,
                tr(ja, "DCオフセット除去", "Remove DC offset"),
            );
            ui.label("");
            ui.end_row();
            number(
                ui,
                tr(ja, "ピーク上限（dBTP）", "Peak limit (dBTP)"),
                &mut s.peak_db,
                -12.0..=-0.1,
                0.1,
            );
            number(
                ui,
                tr(ja, "目標音量（LUFS）", "Target loudness (LUFS)"),
                &mut s.target_lufs,
                -36.0..=-8.,
                0.5,
            );
            ui.end_row();
        });
    ui.separator();
    ui.small(tr(ja,"無音の最小継続時間：先頭・末尾で、この時間以上続く小さい音を無音として除外します。曲中の無音は残します。","Minimum silence: exclude quiet sections of at least this duration at the start/end; keep silence inside songs."));
    ui.small(tr(
        ja,
        "WAV形式はDATの入力に合わせるための設定です。未設定は16bit / 44.1kHzです。",
        "Choose a WAV format matching the DAT input. Default is 16bit / 44.1kHz.",
    ));
}
