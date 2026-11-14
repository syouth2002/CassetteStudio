//! Shared Rust UI for native windows and the browser's WebAssembly canvas.
mod app;
mod appearance;
mod arrangement;
mod feedback;
mod graph;
mod jcard;
mod jcard_view;
#[path = "../../src/model.rs"]
pub mod model;
mod output;
mod platform;
mod play_session;
mod playback;
mod properties;
mod settings;
mod track_drag;
mod transport;
pub use app::Studio;

#[cfg(not(target_arch = "wasm32"))]
pub fn show_startup_error(message: String) {
    rfd::MessageDialog::new()
        .set_title("Cassette Studio")
        .set_description(message)
        .set_level(rfd::MessageLevel::Error)
        .show();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_native(base: String, data_root: std::path::PathBuf) -> eframe::Result {
    let options = eframe::NativeOptions {
        persistence_path: Some(data_root.join("rust-ui.ron")),
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0])
            .with_icon(eframe::egui::IconData {
                rgba: include_bytes!("../../assets/cassette-studio-app.rgba").to_vec(),
                width: 128,
                height: 128,
            }),
        ..Default::default()
    };
    eframe::run_native(
        "Cassette Studio",
        options,
        Box::new(move |cc| Ok(Box::new(Studio::new(cc, base)))),
    )
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub async fn start() -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::JsCast;
    let window = web_sys::window().ok_or("Window unavailable")?;
    let canvas = window
        .document()
        .ok_or("Document unavailable")?
        .get_element_by_id("cassette-canvas")
        .ok_or("Canvas unavailable")?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    let base = format!("{}/", window.location().origin()?);
    eframe::WebRunner::new()
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(move |cc| Ok(Box::new(Studio::new(cc, base)))),
        )
        .await
}
