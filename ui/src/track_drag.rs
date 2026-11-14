//! Track reorder feedback and insertion-boundary semantics.
use crate::{appearance, settings::tr};
use eframe::egui::{self, Color32};

pub struct TrackDrag {
    pub collection: String,
    pub side: usize,
    pub from: usize,
    pub title: String,
    pub artist: String,
}

/// Convert a boundary in the original list to an index after removing the source.
pub fn destination(from: usize, boundary: usize, len: usize) -> Option<usize> {
    if from >= len || boundary > len {
        return None;
    }
    let to = boundary - usize::from(boundary > from);
    (to != from).then_some(to)
}

/// One shared location per gap, at the separator between adjacent rows.
pub fn boundaries(rows: &[(usize, egui::Rect)], end: f32) -> Vec<(usize, f32)> {
    let Some((first, rect)) = rows.first() else {
        return Vec::new();
    };
    let mut result = vec![(*first, rect.top())];
    result.extend(
        rows.windows(2)
            .map(|pair| (pair[1].0, (pair[0].1.bottom() + pair[1].1.top()) * 0.5)),
    );
    result.push((rows.last().unwrap().0 + 1, end));
    result
}

/// Positive moves content down (toward the start); negative moves it up.
pub fn edge_scroll(pointer: egui::Pos2, viewport: egui::Rect) -> f32 {
    if viewport.height() < 1.
        || pointer.x < viewport.left()
        || pointer.x > viewport.right()
        || pointer.y < viewport.top() - 64.
        || pointer.y > viewport.bottom() + 64.
    {
        return 0.;
    }
    let zone = 48_f32.min(viewport.height() * 0.3);
    if pointer.y < viewport.top() + zone {
        600. * ((viewport.top() + zone - pointer.y) / zone).clamp(0., 1.)
    } else if pointer.y > viewport.bottom() - zone {
        -600. * ((pointer.y - viewport.bottom() + zone) / zone).clamp(0., 1.)
    } else {
        0.
    }
}

pub fn preview(ctx: &egui::Context, drag: &TrackDrag, to: Option<usize>, ja: bool) {
    let Some(pointer) = ctx.pointer_hover_pos() else {
        return;
    };
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::new("track-drag-preview"),
    ));
    let screen = ctx.content_rect();
    let width = 310_f32.min(screen.width() - 16.);
    let pos = egui::pos2(
        (pointer.x + 18.)
            .min(screen.right() - width - 8.)
            .max(screen.left() + 8.),
        (pointer.y + 20.)
            .min(screen.bottom() - 90.)
            .max(screen.top() + 8.),
    );
    let rect = egui::Rect::from_min_size(pos, egui::vec2(width, 82.));
    painter.rect_filled(
        rect.translate(egui::vec2(3., 4.)),
        6.,
        Color32::from_black_alpha(35),
    );
    painter.rect_filled(rect, 6., appearance::PAPER);
    painter.rect_stroke(
        rect,
        6.,
        egui::Stroke::new(2_f32, appearance::GREEN),
        egui::StrokeKind::Inside,
    );
    let text_painter = painter.with_clip_rect(rect.shrink(9.));
    for (offset, text, size, color) in [
        (9., drag.title.clone(), 15., appearance::INK),
        (30., drag.artist.clone(), 13., appearance::MUTED),
        (
            53.,
            to.map(|i| {
                format!(
                    "{:02} → {:02}  {}",
                    drag.from + 1,
                    i + 1,
                    tr(ja, "ここに移動", "Move here")
                )
            })
            .unwrap_or_else(|| {
                tr(
                    ja,
                    "移動先の曲間で離してください",
                    "Release between tracks to move",
                )
                .into()
            }),
            13.,
            appearance::GREEN,
        ),
    ] {
        text_painter.text(
            pos + egui::vec2(10., offset),
            egui::Align2::LEFT_TOP,
            text,
            egui::FontId::proportional(size),
            color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gaps_have_one_boundary_and_edge_scroll_is_directional() {
        let rect =
            |top, bottom| egui::Rect::from_min_max(egui::pos2(0., top), egui::pos2(200., bottom));
        let gaps = boundaries(
            &[
                (0, rect(10., 60.)),
                (1, rect(84., 134.)),
                (2, rect(158., 208.)),
            ],
            220.,
        );
        assert_eq!(gaps, vec![(0, 10.), (1, 72.), (2, 146.), (3, 220.)]);
        assert!(boundaries(&[], 0.).is_empty());
        let viewport = rect(100., 500.);
        assert_eq!(edge_scroll(egui::pos2(100., 300.), viewport), 0.);
        assert!(edge_scroll(egui::pos2(100., 110.), viewport) > 0.);
        assert!(edge_scroll(egui::pos2(100., 490.), viewport) < 0.);
        assert_eq!(edge_scroll(egui::pos2(100., 520.), viewport), -600.);
        assert_eq!(edge_scroll(egui::pos2(250., 490.), viewport), 0.);
        assert_eq!(edge_scroll(egui::pos2(100., 600.), viewport), 0.);
    }
    #[test]
    fn insertion_boundaries_account_for_removal_and_no_op() {
        assert_eq!(destination(0, 4, 4), Some(3));
        assert_eq!(destination(3, 0, 4), Some(0));
        assert_eq!(destination(1, 3, 4), Some(2));
        assert_eq!(destination(2, 1, 4), Some(1));
        assert_eq!(destination(2, 2, 4), None);
        assert_eq!(destination(2, 3, 4), None);
        assert_eq!(destination(0, 1, 1), None);
        assert_eq!(destination(4, 0, 4), None);
        assert_eq!(destination(0, 5, 4), None);
    }
}
