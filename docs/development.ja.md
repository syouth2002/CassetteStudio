# 開発環境・ビルド・検証

[トップ](../README.ja.md) · [English](development.md)

通常利用の導入は[インストールガイド](installation.ja.md)、操作と仕様は[使い方](usage.ja.md)にまとめています。このページはソースからのビルドと配布物作成用です。

## Windows / Visual Studio 2026

Rust stable MSVC、Visual Studio 2026の「C++によるデスクトップ開発」、WebView2を用意します。JavaScriptテストとURL取り込みにはNode.js 22以上を使います。初回の依存関係取得にはネット接続が必要です。

```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-tools.ps1
cargo build --release --features desktop --bins --locked
```

セットアップはFFmpeg・FFprobe・yt-dlpを`tools`へ取得してSHA-256を照合し、システムのPATHは変更しません。Node.jsは別途導入します。

`CassetteStudio.sln`を開き、DebugまたはRelease／x64でビルドし、F5またはCtrl+F5で専用ウィンドウを起動できます。Visual StudioはCargoを呼び出す連携で、Rust用言語サービスは同梱しません。

`CassetteStudio.cmd`は必要時にビルドして起動し、`CassetteStudio-Browser.cmd`はブラウザ用です。同じライブラリの既存プロセスは終了してください。

## Ubuntu / Linux

Ubuntu 24.04 x64でRust stableと開発用ライブラリを導入します。

```sh
sudo apt install build-essential pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libxdo-dev libdbus-1-dev libssl-dev ffmpeg
cargo run --release --features desktop --bin cassette-studio-desktop --locked
```

`sh scripts/start.sh`でもデスクトップをビルド・起動できます。ブラウザ用は`sh scripts/start-browser.sh`です。URL用ツールは[導入ガイド](installation.ja.md)を参照してください。

WSL向け`scripts/setup-linux-build.sh`・`scripts/build-linux-wsl.sh`は`/var/tmp`の独立したRust環境でビルドし、`dist/linux-x64`へ実行ファイルを配置します。

## macOS

Xcode Command Line Tools、Rust stable、FFmpegを用意します。

```sh
cargo run --release --features desktop --bin cassette-studio-desktop --locked
sh scripts/package-public-macos.sh 0.1.10
```

現在のCPU用に作成します。Apple SiliconとIntelは別々のMacランナーでビルドします。

## Arch Linux

同じリリースの`PKGBUILD`と`CassetteStudio-0.1.10-Arch-source.tar.gz`を空フォルダーへ置き、内容を確認して通常ユーザーで実行します。

```sh
sudo pacman -S --needed base-devel rust pkgconf python
makepkg -si
```

PKGBUILD内のSHA-256でソースを検証します。テンプレートは`packaging/arch/PKGBUILD.in`で、`scripts/prepare-arch-package.py`が現在のコミットからソーススナップショットとチェックサム付きPKGBUILDを生成します。

## 公開用パッケージ

| 環境 | 作成手順 |
| --- | --- |
| Windows | `scripts/package-public-windows.ps1 -Version 0.1.10` |
| Linux全形式 | ネイティブ版をビルド後、`sh scripts/package-all-linux.sh` |
| macOS | 各CPUのMacで`sh scripts/package-public-macos.sh 0.1.10` |
| Arch | 上記makepkg、または`.github/workflows/arch.yml` |

出力先は`dist/releases/v0.1.10`です。Python 3と、Rustツールチェーン付属の著作権表示がライセンス収集に必要です。

`package-windows.ps1`・`package-linux.sh`は外部ツールを同梱する手元用の補助スクリプトです。公開用のWindows・従来Linuxパッケージとは内容が違うため、公開版の導入手順と混同しないでください。[再配布条件](../THIRD_PARTY_NOTICES.md)も確認してください。

<a name="linux-bundles"></a>

## Linux共通パッケージ

Ubuntu 24.04 x86_64、`dist/linux-x64`のネイティブ実行ファイル、Flatpak・patchelf・mksquashfs・GTK/WebKitGTK開発環境・FFmpegを使います。

```sh
flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install --user flathub org.gnome.Platform//49
python3 scripts/build-linux-bundles.py flatpak --build-dir /var/tmp/cassette-flatpak-build
python3 scripts/build-linux-bundles.py appimage --build-dir /var/tmp/cassette-appimage-build
```

中断したステージの再利用は`--resume`です。ツール取得は`packaging/linux-bundles/tools.lock.json`のSHA-256で検証し、OSパッケージの版と著作権表示を収録します。フォントはアプリ専用設定を使います。

一括生成の`sh scripts/package-all-linux.sh`はDEB・従来版・Flatpak・AppImage・対応ソースを生成します。[同梱ライブラリの対応ソース](linux-bundle-sources.md)も参照してください。

## 実行設定

ブラウザ版は`cargo run --release --bin cassette-studio --locked`で起動します。デスクトップ版も同じローカル音声エンジンを持ちます。

| 環境変数 | 内容 |
| --- | --- |
| `CASSETTE_DATA` | データ保存先を明示指定 |
| `CASSETTE_PORT` | 内部通信ポート。通常版の既定は3210、Flatpak／AppImageの既定は空きポート |
| `CASSETTE_FFMPEG`・`CASSETTE_FFPROBE`・`CASSETTE_YT_DLP`・`CASSETTE_NODE` | 各ツールのフルパス |

ツールは明示指定、実行ファイル横の`tools`、実行ファイルのフォルダー、macOSのHomebrewパス、開発用`tools`、PATHの順に探します。保存先・移行は[保存とバックアップ](usage.ja.md#storage)にまとめています。

<a name="validation"></a>

## 検証

```sh
cargo fmt --check
cargo test --locked
cargo clippy --features desktop --all-targets --locked -- -D warnings
cargo build --locked
node scripts/test-i18n.cjs
node scripts/test-frontend-startup.cjs
node scripts/test-tape-settings.cjs
node scripts/test-downloads.cjs
node scripts/test-reference-tone.cjs
node scripts/test-entry-fit.cjs
```

音声テストにはFFmpeg／FFprobeが必要です。`tools`または環境変数で指定します。

| 対象 | スクリプト |
| --- | --- |
| 音声・動画・無音・フェード・範囲 | `node scripts/smoke-test.mjs` |
| 自動作成・空カセット・重複除外 | `node scripts/test-batch.mjs` |
| 音量・DC・疑似ステレオ | `node scripts/test-loudness.mjs`、`node scripts/test-dc.mjs`、`python scripts/test-pseudo-stereo.py` |
| LUFS | `node scripts/test-lufs-graph.mjs`、`node scripts/test-lufs-side-graph.mjs`、`node scripts/test-lufs-graph-ui.cjs` |
| 無音再解析・取り込み重複・削除 | `node scripts/test-silence-reanalysis.mjs`、`node scripts/test-duplicate-import.mjs`、`node scripts/test-library-delete.mjs` |
| 先頭無音・DAT向けWAV | `node scripts/test-dat-settings.mjs` |
| J-CARD | `node scripts/test-jcard.cjs`、`node scripts/test-jcard-editor.cjs`、`node scripts/test-jcard-api.mjs` |

テストは`work`配下の合成音源と独立ライブラリを使い、通常のライブラリには入りません。

Flatpakをテスト用ユーザー領域へ導入後、`sh scripts/test-linux-bundles.sh --appimage ./dist/linux-bundles/CassetteStudio-0.1.10-x86_64.AppImage`で同時起動・保存先分離・音声処理を検証します。`scripts/test-macos-package.py`・`scripts/test-arch-package.py`は各環境で配布実行ファイルを検証します。[物理機器などの確認範囲](installation.ja.md#coverage)も参照してください。

## ソース構成と調査資料

[ソースコードの役割](architecture.ja.md)に、表示・ボタン・音声処理・API・保存の分担をまとめています。未実装のDolby B/Cについては[調査資料](dolby-b-c-research.ja.md)を参照してください。
