# Development, builds and validation

[Home](../README.md) · [日本語](development.ja.md) · [Source responsibilities](architecture.md)

## Prerequisites

Use stable Rust, Python 3.11+, FFmpeg and FFprobe. Windows needs Visual Studio 2026 C++ desktop tools; macOS needs Xcode Command Line Tools. Linux builds need pkg-config and ALSA, X11/Wayland and EGL development libraries. WebView2/WebKitGTK are not required by the current UI.

## Build the shared UI first

The browser UI is Rust compiled to WebAssembly and embedded in the local server. A fresh checkout must generate it before building the application.

```sh
cargo install wasm-bindgen-cli --version 0.2.129 --locked
python scripts/build-rust-ui-web.py
cargo build --release --features desktop --bins --locked
cargo test --workspace --features desktop --locked
cargo clippy --workspace --features desktop --all-targets -- -D warnings
```

Match wasm-bindgen-cli to Cargo.lock. `target/release/cassette-studio-desktop` (with `.exe` on Windows) is the native application; `cassette-studio` is the local browser engine. The Visual Studio solution invokes Cargo too.

<a id="linux-bundles"></a>
<a id="validation"></a>

## Packages and checks

Windows ZIPs contain the application, tool setup helper, bilingual guides, screenshots and licenses. Build Linux with `scripts/build-linux-wsl.sh`, package DEB/tar.gz with `package-public-linux.sh`, and create Flatpak/AppImage bundles with `build-linux-bundles.py`. Bundles use egui, ALSA and a PulseAudio-compatible audio server. `.github/workflows/release-builds.yml` builds shared WASM assets and native macOS/Arch packages, then runs isolated audio and persistence checks. Arch source snapshots include the generated shared WASM assets.

Tests must use disposable data. `test-rust-ui-browser.py` creates synthetic media and a demo library; `test-rust-ui-*.cjs` cover recording mode, preparation, cache reuse, reordering and J-CARD rotation. Legacy HTML tests apply only to the compatibility view. Physical deck recording still requires checking the actual audio path.
