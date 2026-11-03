# Cassette Studio v0.1.8 / 2026-11-03

## 日本語 — v0.1.7からの変更

- 1チャンネルと、全体の左右差が十分小さい2チャンネル音源のモノラル判定を追加しました。通常のステレオ・逆相・多チャンネル音源は疑似ステレオ化の対象外です。
- 「音量調整」に曲ごとの疑似ステレオ化（OFF／弱／標準／強）を追加しました。初期値はOFFで、同じ曲を使う全カセットに適用し、元ファイルは変更しません。
- 「保存して試聴」を追加しました。疑似ステレオ化後の音量・ピークを測定し、音量統一・DC除去・再生・WAV・曲単位／A・B面全体のLUFSグラフに反映します。
- 状態欄に「モノラル／MONO」と「疑似ステレオ：弱・中・強」を表示します。
- 既存曲の未判定データを起動後にバックグラウンドで確認し、結果を保存します。無音・音量・DC解析はやり直しません。
- 「保存」「保存して試聴」の処理中メッセージがダイアログの後ろに隠れる問題を修正しました。
- Windows／Ubuntu／Linux従来版・Flatpak・AppImage／macOS Apple Silicon・Intel／Arch Linuxの配布物と、日英ドキュメント・画面画像を更新しました。

FlatpakはGNOME 49ランタイムを使用します。AppImageはglibc 2.39以降のUbuntu 24.04互換環境が対象です。新形式の検証はUbuntu 24.04／WSL2の仮想画面で行い、物理音声機器・実際のWayland・D&D／保存ポータルの組み合わせは追加検証が必要です。macOS版はアドホック署名で、Developer ID署名・公証は行っていません。

## English — Changes since v0.1.7

- Added mono detection for single-channel audio and two-channel audio with negligible difference across the complete source. Stereo, antiphase and multichannel sources cannot enable pseudo stereo.
- Added per-track pseudo stereo (OFF / Weak / Standard / Strong) under Adjust loudness. Default is OFF; settings are shared by every cassette using that track and originals remain unchanged.
- Added Save and audition. Processed loudness and true peak measurements support normalization, DC removal, playback, WAV exports and track/whole-side LUFS graphs.
- Added MONO and pseudo stereo strength badges in the status column.
- Existing unclassified tracks are checked in the background after startup and results are saved, without rerunning silence, loudness or DC analysis.
- Fixed progress messages appearing behind the audio settings dialog during Save or Save and audition.
- Updated all Windows, Ubuntu, traditional Linux/Flatpak/AppImage, macOS Apple Silicon/Intel and Arch Linux packages, bilingual documentation and screenshots.

Flatpak uses GNOME 49. AppImage targets Ubuntu 24.04-compatible systems with glibc 2.39+. New-format validation uses Ubuntu 24.04 / WSL2 with a virtual display; physical audio, real Wayland and drag/drop/save-portal combinations need additional checks. macOS uses ad-hoc signing without Developer ID signing or notarization.
