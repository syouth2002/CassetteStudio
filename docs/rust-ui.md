# Rust UI (development version)

[Home](../README.md) · [日本語](rust-ui.ja.md)

The UI now uses Rust's egui/eframe instead of HTML/JavaScript in a WebView. Native Windows/Linux/macOS windows and the WebAssembly browser canvas share UI code. FFmpeg still handles media import, analysis and WAV rendering. Audacity is not used.

Version 0.2.0 packages and screenshots use the shared Rust UI.

## Build and run

Windows needs Rust stable MSVC and Visual Studio 2026's Desktop development with C++. WebView2 is no longer required.

```sh
cargo run --features desktop --bin cassette-studio-desktop
```

Linux needs a C/C++ toolchain, pkg-config, X11/Wayland, OpenGL and ALSA development libraries. Add `libasound2-dev` and the `libxkbcommon-x11-0` runtime to the existing Ubuntu setup; `libegl1` and `libgl1` are also required. macOS uses Rust and Xcode Command Line Tools.

For the browser, build the Rust UI before the server. The wasm-bindgen CLI must match Cargo.lock; the build script reports the required version.

```sh
cargo install wasm-bindgen-cli --version 0.2.129 --locked
python scripts/build-rust-ui-web.py
cargo run --bin cassette-studio
```

Open the printed local URL. HTML only hosts the canvas and loader; Rust owns controls, drawing, graphs and J-CARD interactions. The browser still uses the local FFmpeg-backed engine; it does not run FFmpeg independently.

## Controls

- Top: add media files, add URLs and Options. Music/video drag and drop is supported.
- Left: all tracks, playlists and cassettes, in a resizable pane.
- Right: tracks, A/B sides, recording settings, J-CARD, LUFS and WAV output. Select with checkboxes; reorder with arrows or dragging.
- Bottom: play, pause, stop, seek and volume. Native builds also offer output-device selection.
- Options: automatic/Japanese/English, reference tone, selected-track loudness/DC and silence analysis. Automatic uses Japanese only for Japanese environments, otherwise English.
- J-CARD: outside/inside, text/images/rectangles, size/rotation/position, standard/custom spine, fold-out panels, editable files, SVG and two-sided PDF. Preview and export share SVG rendering. Closing waits for autosave.

Browser exports open a download link; click it to retrieve the file.

## Data and code

Existing library, media and J-CARD formats and storage locations are retained. Native UI settings use `rust-ui.ron` inside the same data folder; browser UI settings use browser storage. Previous UI preferences are separate, so review language and creation settings on first use.

`ui/src/` separates the main view, transport, playback, settings, graphs and J-CARD. The audio engine, planner and storage APIs remain shared. `/legacy/` retains the previous view for compatibility checks during migration. Native Rust windows do not load it or use a WebView.

Checks: `cargo test --workspace --features desktop`, `cargo clippy --workspace --features desktop --all-targets -- -D warnings`, and `cargo check -p cassette-ui --target wasm32-unknown-unknown`. Existing synthetic-media integration tests continue to check audio processing.
