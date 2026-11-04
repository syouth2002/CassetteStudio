# Cassette Studio v0.1.10 / 2026-11-04

## 日本語 — v0.1.9からの変更

- J-CARDの背幅を「標準（12mm）」と「カスタム」から選択できるようにしました。標準では12mmに設定し、カスタムではケースの実測値を入力できます。メーカーや製品で幅が異なるスリムケースにも対応できます。
- 保存済みJ-CARDの背幅は保持し、12mm以外ならカスタムとして開きます。サイズ変更は既存の文字・画像を自動拡大縮小しません。
- 日英ドキュメントを「導入 → 使い方・仕様 → J-CARD → 開発者向け」の順に整理し、関連する説明を各ガイドに集約しました。
- J-CARDの説明に音量調整画面が表示されていた画像を修正し、自動選曲の説明とJ-CARDの説明の混在を解消しました。背幅の設定を含む画面画像と日英ガイドを更新しました。
- Windows／Ubuntu／Linux従来版・Flatpak・AppImage／macOS Apple Silicon・Intel／Arch Linuxの配布物を更新しました。

FlatpakはGNOME 49を使用します。AppImageはglibc 2.39以降のUbuntu 24.04互換環境が対象です。Linux共通パッケージはUbuntu 24.04／WSL2の仮想画面で検証しています。物理音声機器・実際のWayland・D&D／保存ポータルの組み合わせは追加検証が必要です。macOS版はアドホック署名で、Developer ID署名・公証は行っていません。

## English — Changes since v0.1.9

- J-CARD spine width now offers Standard (12 mm) and Custom. Standard sets 12 mm; Custom accepts the measured width, including slim cases whose dimensions vary by manufacturer or product.
- Saved spine widths are preserved; widths other than 12 mm open as Custom. Dimension changes do not scale existing text or images.
- Reorganized bilingual documentation into installation, usage/specifications, J-CARD and development guides, keeping related information together.
- Corrected the J-CARD image that showed the loudness dialog and separated J-CARD instructions from automatic track selection. Updated screenshots and bilingual instructions for spine width selection.
- Updated Windows, Ubuntu, traditional Linux/Flatpak/AppImage, macOS Apple Silicon/Intel and Arch Linux packages.

Flatpak uses GNOME 49. AppImage targets Ubuntu 24.04-compatible systems with glibc 2.39+. Linux bundle validation uses Ubuntu 24.04 / WSL2 with a virtual display; physical audio, real Wayland and drag/drop/save-portal combinations need additional checks. macOS uses ad-hoc signing without Developer ID signing or notarization.
