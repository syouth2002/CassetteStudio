//! Display-only J-CARD navigation. Coordinates in the document stay in mm.
use eframe::egui::Vec2;
pub const NORMAL: f32 = 96. / 25.4;
pub const MIN: f32 = 0.25;
pub const MAX: f32 = 12.;
pub fn zoom_at(pan: Vec2, old: f32, new: f32, anchor: Vec2) -> Vec2 {
    anchor - (anchor - pan) * (new / old)
}
pub fn rotated_size(size: Vec2, turns: u8) -> Vec2 {
    if turns.is_multiple_of(2) {
        size
    } else {
        Vec2::new(size.y, size.x)
    }
}
pub fn to_view(p: Vec2, size: Vec2, turns: u8) -> Vec2 {
    match turns % 4 {
        1 => Vec2::new(size.y - p.y, p.x),
        2 => size - p,
        3 => Vec2::new(p.y, size.x - p.x),
        _ => p,
    }
}
pub fn from_view(p: Vec2, size: Vec2, turns: u8) -> Vec2 {
    to_view(p, rotated_size(size, turns), (4 - turns % 4) % 4)
}
pub fn delta_from_view(delta: Vec2, turns: u8) -> Vec2 {
    match turns % 4 {
        1 => Vec2::new(delta.y, -delta.x),
        2 => -delta,
        3 => Vec2::new(-delta.y, delta.x),
        _ => delta,
    }
}
pub fn fit(page: Vec2, viewport: Vec2) -> (f32, Vec2) {
    let room = (viewport - Vec2::splat(48.)).max(Vec2::splat(1.));
    let zoom = (room.x / page.x).min(room.y / page.y).clamp(MIN, MAX);
    (zoom, (viewport - page * zoom) * 0.5)
}
/// Keep a visible patch of the card inside the editor, in logical screen pixels.
/// Small cards/viewports retain as much of that patch as they can contain.
pub fn constrain_pan(pan: Vec2, page: Vec2, viewport: Vec2) -> Vec2 {
    let page = page.max(Vec2::ZERO);
    let viewport = viewport.max(Vec2::ZERO);
    let visible = Vec2::splat(32.).min(page).min(viewport);
    pan.max(visible - page).min(viewport - visible)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quarter_turn_coordinates_and_drag_deltas_round_trip() {
        let size = Vec2::new(102., 100.);
        for turns in 0..4 {
            for point in [Vec2::ZERO, size, Vec2::new(12., 37.)] {
                assert!(
                    (from_view(to_view(point, size, turns), size, turns) - point).length() < 0.001
                );
                let delta = Vec2::new(8., -6.);
                let view_delta = to_view(point + delta, size, turns) - to_view(point, size, turns);
                assert!((delta_from_view(view_delta, turns) - delta).length() < 0.001);
            }
        }
        assert_eq!(rotated_size(size, 1), Vec2::new(100., 102.));
        assert_eq!(to_view(Vec2::ZERO, size, 1), Vec2::new(100., 0.));
    }
    #[test]
    fn panning_keeps_a_visible_patch_in_every_direction() {
        for page in [
            Vec2::new(400., 300.),
            Vec2::new(1800., 1200.),
            Vec2::new(20., 15.),
        ] {
            let viewport = Vec2::new(600., 440.);
            for requested in [
                Vec2::splat(-10000.),
                Vec2::splat(10000.),
                Vec2::new(-10000., 10000.),
            ] {
                let pan = constrain_pan(requested, page, viewport);
                let visible = egui_intersection(pan, page, viewport);
                assert!(visible.x >= 32_f32.min(page.x) - 0.001);
                assert!(visible.y >= 32_f32.min(page.y) - 0.001);
            }
        }
        assert_eq!(
            constrain_pan(
                Vec2::new(30., 40.),
                Vec2::new(400., 300.),
                Vec2::new(600., 440.)
            ),
            Vec2::new(30., 40.)
        );
        let tiny = Vec2::new(12., 8.);
        let pan = constrain_pan(Vec2::splat(10000.), Vec2::splat(500.), tiny);
        assert_eq!(egui_intersection(pan, Vec2::splat(500.), tiny), tiny);
    }
    fn egui_intersection(pan: Vec2, page: Vec2, viewport: Vec2) -> Vec2 {
        (pan + page).min(viewport) - pan.max(Vec2::ZERO)
    }
    #[test]
    fn zoom_keeps_cursor_on_same_document_point() {
        let pan = Vec2::new(-43., 27.);
        let anchor = Vec2::new(241., 119.);
        let before = (anchor - pan) / 4.;
        let after = zoom_at(pan, 4., 7., anchor);
        assert!(((anchor - after) / 7. - before).length() < 0.001);
    }
    #[test]
    fn fit_centers_and_contains_extended_card() {
        let page = Vec2::new(260., 100.);
        let viewport = Vec2::new(600., 440.);
        let (z, p) = fit(page, viewport);
        assert!(p.x >= 23.9 && p.y >= 23.9);
        assert!((page * z + p * 2. - viewport).length() < 0.001);
    }
}
