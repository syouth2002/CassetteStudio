# Cassette Studio v0.1.9 / 2026-11-03

## 日本語 — v0.1.8からの変更

- 「自動でカセット作成」で、新規カセットを作る場合も既存カセットのA・B面に収録済みの曲を除外するようにしました。用意済みの空カセットへの自動配置と同じ選曲ルールになります。
- 選択曲にも収録済みの除外を適用し、未収録の曲がなければメッセージを表示します。手動作成での曲の再利用と、同じカセット内のA面からB面への引き継ぎは維持します。
- 自動作成前に保留中の編集を保存し、新規カセットには実際に配置した曲だけを記録します。
- 新規J-CARDのミックスリストは外側だけに自動配置し、内側を白紙・白背景にしました。内側への手動配置・編集と、既存の保存済みデザインは保持します。
- 日英ドキュメントと、自動配置・J-CARDの画面画像を更新しました。
- Windows／Ubuntu／Linux従来版・Flatpak・AppImage／macOS Apple Silicon・Intel／Arch Linuxの配布物を更新しました。

FlatpakはGNOME 49を使用します。AppImageはglibc 2.39以降のUbuntu 24.04互換環境が対象です。Linux共通パッケージはUbuntu 24.04／WSL2の仮想画面で検証しています。物理音声機器・実際のWayland・D&D／保存ポータルの組み合わせは追加検証が必要です。macOS版はアドホック署名で、Developer ID署名・公証は行っていません。

## English — Changes since v0.1.8

- Automatic creation of a new cassette now excludes tracks already recorded on either side of existing cassettes, matching automatic filling of prepared empty tapes.
- Selected candidates are filtered too, with a message when no unrecorded tracks remain. Manual reuse and A-to-B continuation inside one cassette are retained.
- Pending edits are saved before automatic planning; new automatic cassettes record only the tracks actually placed.
- New J-CARDs place track lists on the outside only and start with a blank white inside. Manual inside editing and existing saved artwork are preserved.
- Updated bilingual documentation and automatic-arrangement/J-CARD screenshots.
- Updated Windows, Ubuntu, traditional Linux/Flatpak/AppImage, macOS Apple Silicon/Intel and Arch Linux packages.

Flatpak uses GNOME 49. AppImage targets Ubuntu 24.04-compatible systems with glibc 2.39+. Linux bundle validation uses Ubuntu 24.04 / WSL2 with a virtual display; physical audio, real Wayland and drag/drop/save-portal combinations need additional checks. macOS uses ad-hoc signing without Developer ID signing or notarization.
