# 開発環境・ビルド・検証

[トップ](../README.ja.md) · [English](development.md) · [ソース構成](architecture.ja.md)

## 必要な環境

Rust stable、Python 3.11以降、FFmpeg／FFprobeを用意します。WindowsはVisual Studio 2026のC++デスクトップ開発環境、macOSはXcode Command Line Toolsが必要です。Linuxではpkg-configとALSA・X11／Wayland・EGLの開発ライブラリを使用します。WebView2／WebKitGTKは新UIの要件ではありません。

## 共通UIとアプリをビルド

ブラウザ画面もRustで描画し、生成したWebAssemblyをサーバーへ埋め込みます。**新規チェックアウトでは先に共通UIをビルドしてください。**

```sh
cargo install wasm-bindgen-cli --version 0.2.129 --locked
python scripts/build-rust-ui-web.py
cargo build --release --features desktop --bins --locked
cargo test --workspace --features desktop --locked
cargo clippy --workspace --features desktop --all-targets -- -D warnings
```

`wasm-bindgen-cli`はCargo.lockと同じバージョンを使います。Windowsでは`target/release/cassette-studio-desktop.exe`、Linux/macOSでは同名の拡張子なし実行ファイルが専用ウィンドウ版です。`cassette-studio`はブラウザ用ローカルエンジンです。Visual StudioのソリューションもCargoを呼び出します。

## 配布物

- Windows：実行ファイル・`SetupAudioTools.cmd`・日英ガイド・画像・ライセンスをZIP化します。
- Ubuntu/Linux：`scripts/build-linux-wsl.sh`でビルドし、`scripts/package-public-linux.sh`でDEBとtar.gzを作成します。
- Flatpak/AppImage：`scripts/build-linux-bundles.py`でネイティブLinux実行ファイルと固定版外部ツールを梱包します。GUIはegui、音声はALSAとPulseAudio互換サーバーを利用します。
- macOS／Arch：`.github/workflows/release-builds.yml`で共通WASMを作成し、各OSのネイティブ環境でビルドと音声・保存APIの検証を行います。Archソースには再現用の共通WASMを含めます。

検証用データは一時フォルダーに作成します。実際のライブラリへテストを実行しないでください。`test-rust-ui-browser.py`は合成音源と架空のライブラリを作り、Playwrightで共通Rust画面を検証します。録音専用モード、再生準備、キャッシュ、曲順変更、J-CARD回転の回帰テストは`test-rust-ui-*.cjs`にあります。旧HTML画面向けテストは互換画面用です。
