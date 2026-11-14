//! Read-only technical information and shared analysis, with recording details.
use crate::{model::*, settings::tr};
use eframe::egui::{self, Ui};
use serde_json::Value;
pub struct Properties {
    pub track: Track,
    pub entry: Option<Entry>,
    pub settings: Settings,
    pub media: Option<Value>,
    pub error: Option<String>,
}
fn row(ui: &mut Ui, label: &str, value: impl Into<String>) {
    ui.label(label);
    ui.add(egui::Label::new(value.into()).wrap().selectable(true));
    ui.end_row();
}
fn measured(n: Option<f64>, unit: &str, ja: bool) -> String {
    n.map(|n| format!("{n:.2} {unit}"))
        .unwrap_or_else(|| tr(ja, "不明／未解析", "Unknown / not analyzed").into())
}
impl Properties {
    pub fn ui(&self, ui: &mut Ui, ja: bool) {
        let t = &self.track;
        let height = (ui.ctx().content_rect().height() - 160.).clamp(240., 560.);
        egui::ScrollArea::vertical()
            .min_scrolled_height(height)
            .max_height(height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.columns(2, |cols| {
                    let left = &mut cols[0];
                    left.heading(tr(ja, "音源情報", "Source information"));
                    egui::Grid::new("source-properties")
                        .num_columns(2)
                        .max_col_width(170.)
                        .striped(true)
                        .show(left, |ui| {
                            row(ui, tr(ja, "曲名", "Title"), t.title.clone());
                            row(ui, tr(ja, "アーティスト", "Artist"), t.artist.clone());
                            row(ui, tr(ja, "ファイル名", "Filename"), t.filename.clone());
                            row(
                                ui,
                                tr(ja, "元の長さ", "Original duration"),
                                format!("{:.2} s", t.duration),
                            );
                            if let Some(m) = &self.media {
                                let unknown = tr(ja, "不明", "Unknown");
                                for (key, label) in [
                                    ("album", tr(ja, "アルバム", "Album")),
                                    (
                                        "album_artist",
                                        tr(ja, "アルバムアーティスト", "Album artist"),
                                    ),
                                    ("track", tr(ja, "トラック番号", "Track number")),
                                    ("disc", tr(ja, "ディスク番号", "Disc number")),
                                    ("genre", tr(ja, "ジャンル", "Genre")),
                                    ("date", tr(ja, "年／日付", "Year / date")),
                                    ("composer", tr(ja, "作曲者", "Composer")),
                                ] {
                                    if let Some(v) = m["tags"][key].as_str() {
                                        row(ui, label, v);
                                    }
                                }
                                row(
                                    ui,
                                    tr(ja, "形式", "Container"),
                                    m["container_name"]
                                        .as_str()
                                        .or_else(|| m["format"].as_str())
                                        .unwrap_or(unknown)
                                        .to_owned(),
                                );
                                if let Some(video) = m["has_video"].as_bool() {
                                    row(
                                        ui,
                                        tr(ja, "保存ファイルの内容", "Stored media"),
                                        if video {
                                            tr(ja, "音声＋映像", "Audio + video")
                                        } else {
                                            tr(ja, "音声のみ", "Audio only")
                                        },
                                    );
                                }
                                if m["has_cover_art"] == true {
                                    row(
                                        ui,
                                        tr(ja, "カバー画像", "Cover art"),
                                        tr(ja, "あり", "Yes"),
                                    );
                                }
                                row(
                                    ui,
                                    tr(ja, "コーデック", "Codec"),
                                    m["codec"].as_str().unwrap_or(unknown).to_uppercase(),
                                );
                                row(
                                    ui,
                                    tr(ja, "サンプリング周波数", "Sample rate"),
                                    m["sample_rate"]
                                        .as_u64()
                                        .map(|v| format!("{} kHz", v as f64 / 1000.))
                                        .unwrap_or(unknown.into()),
                                );
                                row(
                                    ui,
                                    tr(ja, "ビット深度", "Bit depth"),
                                    m["bits"]
                                        .as_u64()
                                        .map(|v| {
                                            format!(
                                                "{v} bit{}",
                                                if m["float_pcm"] == true {
                                                    " (float)"
                                                } else {
                                                    ""
                                                }
                                            )
                                        })
                                        .unwrap_or_else(|| {
                                            if m["bit_depth_applicable"] == false {
                                                tr(
                                                    ja,
                                                    "対象外（圧縮音源）",
                                                    "Not applicable (lossy)",
                                                )
                                                .into()
                                            } else {
                                                unknown.into()
                                            }
                                        }),
                                );
                                row(
                                    ui,
                                    tr(ja, "チャンネル", "Channels"),
                                    format!(
                                        "{} · {}",
                                        m["channels"]
                                            .as_u64()
                                            .map(|v| v.to_string())
                                            .unwrap_or(unknown.into()),
                                        m["channel_layout"].as_str().unwrap_or(unknown)
                                    ),
                                );
                                row(
                                    ui,
                                    tr(ja, "ビットレート", "Bitrate"),
                                    m["bit_rate"]
                                        .as_u64()
                                        .map(|v| format!("{:.0} kbps", v as f64 / 1000.))
                                        .unwrap_or(unknown.into()),
                                );
                                row(
                                    ui,
                                    tr(ja, "ファイルサイズ", "File size"),
                                    m["size"]
                                        .as_u64()
                                        .map(|v| {
                                            format!("{:.2} MiB ({v} bytes)", v as f64 / 1048576.)
                                        })
                                        .unwrap_or(unknown.into()),
                                );
                            }
                        });
                    if let Some(e) = &self.error {
                        left.colored_label(egui::Color32::DARK_RED, e);
                    } else if self.media.is_none() {
                        left.horizontal(|ui| {
                            ui.spinner();
                            ui.label(tr(ja, "音源情報を取得中…", "Reading source information…"));
                        });
                    }
                    let right = &mut cols[1];
                    right.heading(tr(ja, "解析結果・曲の設定", "Analysis / track settings"));
                    egui::Grid::new("analysis-properties")
                        .num_columns(2)
                        .max_col_width(170.)
                        .striped(true)
                        .show(right, |ui| {
                            row(
                                ui,
                                tr(ja, "元音量", "Source loudness"),
                                measured(
                                    t.loudness.as_ref().and_then(|a| a.integrated_lufs),
                                    "LUFS",
                                    ja,
                                ),
                            );
                            row(
                                ui,
                                tr(ja, "トゥルーピーク", "True peak"),
                                measured(
                                    t.loudness.as_ref().and_then(|a| a.true_peak_db),
                                    "dBTP",
                                    ja,
                                ),
                            );
                            row(
                                ui,
                                tr(ja, "DCオフセット L / R", "DC offset L / R"),
                                t.dc.as_ref()
                                    .map(|d| {
                                        format!(
                                            "{:+.4}% / {:+.4}%",
                                            d.offset[0] * 100.,
                                            d.offset[1] * 100.
                                        )
                                    })
                                    .unwrap_or_else(|| tr(ja, "未解析", "Not analyzed").into()),
                            );
                            if let Some(d) = &t.dc {
                                row(
                                    ui,
                                    tr(ja, "DC除去後の音量", "After DC removal"),
                                    measured(d.corrected_loudness.integrated_lufs, "LUFS", ja),
                                );
                                row(
                                    ui,
                                    tr(ja, "DC除去後のピーク", "Peak after DC removal"),
                                    measured(d.corrected_loudness.true_peak_db, "dBTP", ja),
                                );
                            }
                            row(
                                ui,
                                tr(ja, "モノラル判定", "Mono analysis"),
                                match t.mono {
                                    Some(true) => tr(ja, "モノラル", "MONO"),
                                    Some(false) => tr(ja, "非モノラル", "Not mono"),
                                    None => tr(ja, "未解析", "Not analyzed"),
                                },
                            );
                            row(
                                ui,
                                tr(ja, "先頭の無音", "Leading silence"),
                                format!("{:.2} s", t.trim_start),
                            );
                            row(
                                ui,
                                tr(ja, "末尾の無音", "Trailing silence"),
                                format!("{:.2} s", (t.duration - t.trim_end).max(0.)),
                            );
                            row(
                                ui,
                                tr(ja, "無音除去後の長さ", "Trimmed duration"),
                                format!("{:.2} s", t.audible()),
                            );
                            row(
                                ui,
                                tr(ja, "手動音量補正", "Manual gain"),
                                format!("{:+.1} dB", t.manual_gain_db),
                            );
                            row(
                                ui,
                                tr(ja, "疑似ステレオ", "Pseudo stereo"),
                                if ja {
                                    ["OFF", "弱", "中", "強"]
                                } else {
                                    ["OFF", "Weak", "Medium", "Strong"]
                                }[t.pseudo_stereo.min(3) as usize],
                            );
                            if let Some(a) = &t.pseudo_analysis {
                                row(
                                    ui,
                                    tr(ja, "疑似ステレオ後の音量", "Pseudo stereo loudness"),
                                    measured(a.loudness.integrated_lufs, "LUFS", ja),
                                );
                                row(
                                    ui,
                                    tr(ja, "疑似ステレオ後のピーク", "Pseudo stereo peak"),
                                    measured(a.loudness.true_peak_db, "dBTP", ja),
                                );
                            }
                        });
                    if let Some(e) = &self.entry {
                        right.separator();
                        right.heading(tr(ja, "このミックスリストの処理", "This mix / recording"));
                        egui::Grid::new("recording-properties")
                            .num_columns(2)
                            .max_col_width(170.)
                            .striped(true)
                            .show(right, |ui| {
                                row(
                                    ui,
                                    tr(ja, "開始位置／長さ", "Start / duration"),
                                    format!("{:.2} s / {:.2} s", e.offset, e.duration),
                                );
                                row(
                                    ui,
                                    tr(ja, "フェードイン／アウト", "Fade in / out"),
                                    format!("{:.1} s / {:.1} s", e.fade_in, e.fade_out),
                                );
                                row(
                                    ui,
                                    tr(ja, "A面からの引き継ぎ", "Continued from A"),
                                    if e.continuation {
                                        tr(ja, "あり", "Yes")
                                    } else {
                                        tr(ja, "なし", "No")
                                    },
                                );
                                row(
                                    ui,
                                    tr(ja, "出力WAV", "Output WAV"),
                                    format!(
                                        "16 bit / {} kHz",
                                        self.settings.output_sample_rate() as f64 / 1000.
                                    ),
                                );
                                row(
                                    ui,
                                    tr(ja, "音量統一", "Normalization"),
                                    if self.settings.normalize { "ON" } else { "OFF" },
                                );
                                row(
                                    ui,
                                    tr(ja, "DC除去", "DC removal"),
                                    if self.settings.remove_dc { "ON" } else { "OFF" },
                                );
                                row(
                                    ui,
                                    tr(ja, "目標音量／ピーク上限", "Target / peak ceiling"),
                                    format!(
                                        "{:.1} LUFS / {:.1} dBTP",
                                        self.settings.target_lufs, self.settings.peak_db
                                    ),
                                );
                            });
                        for value in crate::appearance::status(t, e, &self.settings, ja) {
                            right.label(value);
                        }
                    }
                });
                ui.separator();
                if let Some(path) = self.media.as_ref().and_then(|m| m["stored_path"].as_str()) {
                    ui.label(tr(ja, "保存場所", "Stored file"));
                    ui.add(egui::Label::new(path).wrap().selectable(true));
                    if ui
                        .button(tr(ja, "保存場所をコピー", "Copy stored path"))
                        .clicked()
                    {
                        ui.ctx().copy_text(path.into());
                    }
                }
                if t.source.starts_with("https://") || t.source.starts_with("http://") {
                    ui.label(tr(ja, "取り込み元URL", "Source URL"));
                    ui.add(egui::Label::new(&t.source).wrap().selectable(true));
                    if ui.button(tr(ja, "URLをコピー", "Copy URL")).clicked() {
                        ui.ctx().copy_text(t.source.clone());
                    }
                }
            });
    }
}
