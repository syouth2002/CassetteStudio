# Rust製UI

[トップ](../README.ja.md) · [English](rust-ui.md)

画面をHTML・JavaScriptとWebViewから、Rustのegui／eframeによる描画へ変更しました。Windows・Linux・macOSの独立したウィンドウと、WebAssemblyによるブラウザ画面で同じUIコードを使います。音源の取り込み・解析・WAV生成には引き続きFFmpegを使い、Audacityは導入しません。

v0.2.0から共通のRust UIを使用します。配布物と画面画像も新UIに更新しています。

## 起動

WindowsではVisual Studio 2026のC++開発環境とRust stable MSVCを用意します。WebView2は不要です。

```powershell
cargo run --features desktop --bin cassette-studio-desktop
```

LinuxのビルドにはC/C++ツールチェーン、pkg-config、X11／Wayland・OpenGL・ALSAの開発ライブラリが必要です。Ubuntuでは既存開発環境に`libasound2-dev`と、実行時の`libxkbcommon-x11-0`を追加します。OpenGL用の`libegl1`・`libgl1`も必要です。macOSはXcode Command Line ToolsとRustを使います。

ブラウザ版は次の順にビルドします。`wasm-bindgen-cli`はCargo.lockの`wasm-bindgen`と同じバージョンにしてください。スクリプトは必要なバージョンを表示します。

```sh
cargo install wasm-bindgen-cli --version 0.2.129 --locked
python scripts/build-rust-ui-web.py
cargo run --bin cassette-studio
```

表示されたローカルURLを開きます。HTMLはキャンバスと起動処理だけを持ち、画面・ボタン・グラフ・J-CARDの操作はRustです。ブラウザ単体でFFmpegを動かす方式ではなく、同じローカル音声処理エンジンを利用します。

## 操作

- 上部：ファイル追加・URL追加・オプション。音楽・動画のD&Dにも対応します。
- 左ペイン：全曲・プレイリスト・カセット。ペイン幅を変更できます。
- 右ペイン：曲一覧、A/B面、録音設定、J-CARD、LUFS、WAV出力。曲のチェック欄で複数選択し、矢印またはドラッグで曲順を変更します。
- 下部：再生・一時停止・停止・シーク・音量。デスクトップ版は出力デバイスも選べます。
- オプション：自動／日本語／英語と基準音。選択曲の解析は右クリックメニューから実行します。自動選択では日本語環境だけ日本語、それ以外は英語です。
- J-CARD：外側／内側、文字・画像・四角形、位置・サイズ・回転、背幅の標準／カスタム、折り返し、編集データ・SVG・両面PDF。プレビューと出力は共通のSVG描画を使います。「閉じる」は自動保存の完了を待ちます。

ブラウザ版の書き出しは、続いて表示されるダウンロードリンクを押すとファイルを取得できます。

## データとコード

ライブラリ・音源・J-CARDの既存の保存形式と保存先を維持します。デスクトップの画面設定は同じデータフォルダーの`rust-ui.ron`、ブラウザの画面設定はそのブラウザの保存領域に入ります。旧UIの表示設定は別なので、言語や作成設定は初回に確認してください。

`ui/src/`を表示・通信・再生・録音設定・グラフ・J-CARDに分けています。`src/audio/`・`src/planner.rs`・保存APIは両UIで共通です。移行中の互換確認用に旧画面を`/legacy/`に残しています。新デスクトップ版は旧画面やWebViewを使用しません。

チェック：`cargo test --workspace --features desktop`、`cargo clippy --workspace --features desktop --all-targets -- -D warnings`、`cargo check -p cassette-ui --target wasm32-unknown-unknown`。音声処理は従来の合成音源テストでも確認します。
