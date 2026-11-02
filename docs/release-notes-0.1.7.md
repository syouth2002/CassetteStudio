# Cassette Studio v0.1.7 / 2026-11-02

## 日本語

- Linux向けにFlatpak版とAppImage版を追加しました。FlatpakはGNOME 49の共通実行環境を使い、AppImageは必要なライブラリをまとめます。現在はx86_64向けです。
- 新形式にFFmpeg・FFprobe・yt-dlp・Node.jsを同梱し、OS側のツールを置換せずに使えるようにしました。
- 通常版・Flatpak版・AppImage版の保存先を分け、新形式では空いている内部通信ポートを選びます。既存のライブラリを変更せず、別々のデータで同時起動できます。
- FlatpakのURL取り込みツールを展開済みの形式で収録し、実行環境内で動作するようにしました。
- 日本語と操作記号のフォントを同梱し、アプリ専用設定で参照します。OS全体のフォント設定は変更しません。
- Linux用WebViewを正しいGTKコンテナーへ配置し、空白画面になる問題を修正しました。
- Linux従来版・Flatpak・AppImageをまとめて生成する手順と、同時起動・音声処理・URL取り込みの検証を追加しました。
- Windows、Ubuntu DEB、Linux従来版／Flatpak／AppImage、macOS（Apple Silicon／Intel）、Arch Linuxの配布物、日英ドキュメントと画面画像を更新しました。
- 同梱ライブラリの著作権表示と対応ソースを提供します。ソース用アーカイブは通常利用には不要です。

Flatpakの初回導入にはランタイムの取得が必要です。AppImageはUbuntu 24.04相当・glibc 2.39以降が対象で、全ディストリビューションへの対応を保証するものではありません。新形式はUbuntu 24.04／WSL2と仮想画面で検証しています。物理音声機器・実際のWayland環境・D&D／保存ポータルの組み合わせは追加検証が必要です。macOS版はアドホック署名で、Developer ID署名・公証は行っていません。

## English

- Added x86_64 Flatpak and AppImage packages. Flatpak uses the common GNOME 49 runtime; AppImage bundles required libraries.
- Bundled FFmpeg, FFprobe, yt-dlp and Node.js without replacing host tools.
- Separated native/Flatpak/AppImage libraries and selected free internal ports for the new formats, allowing simultaneous use without modifying existing libraries.
- Packaged yt-dlp in unpacked form so URL imports work inside Flatpak.
- Bundled Japanese and symbol fonts using application-specific configuration, leaving global font settings unchanged.
- Fixed Linux WebView placement into the correct GTK container, preventing a blank desktop window.
- Added combined Linux packaging and synthetic coexistence/audio/URL tests.
- Updated Windows, Ubuntu DEB, traditional Linux/Flatpak/AppImage, macOS (Apple Silicon/Intel) and Arch Linux downloads, bilingual documentation and screenshots.
- Provided bundled-library notices and corresponding-source archives; source downloads are optional for ordinary users.

Flatpak needs its runtime on first installation. AppImage targets Ubuntu 24.04-compatible systems with glibc 2.39+ and does not guarantee all distributions. New-format validation is on Ubuntu 24.04 / WSL2 with a virtual display. Physical audio, real Wayland sessions and drag/drop/save-portal combinations need further validation. macOS uses ad-hoc signatures without Developer ID signing or notarization.
