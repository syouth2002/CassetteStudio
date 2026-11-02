# Cassette Studio v0.1.6 / 2026-11-02

## 日本語

- 曲を1曲選択して表示するLUFSグラフを追加しました。Short-term（3秒）とMomentary（400ms）を切り替えられます。
- 補正前／録音設定適用後の推移、目標LUFS線、再生範囲全体のLUFS・トゥルーピークを表示します。カセット内の曲は途中再生とフェードも反映します。
- A/B面全体のLUFSグラフを追加しました。先頭・曲間・末尾の無音、曲順、再生範囲、フェードを含めて解析し、同じ画面でA面・B面を切り替えられます。空の面も表示できます。
- 面全体はWAV生成と同じ処理で組み立てて解析し、曲の切り替わりをまたぐ測定窓・ゲートを維持します。曲の開始位置を番号・曲名で表示します。
- 解析結果を保存・再利用し、音源や再生範囲、曲順、設定が変わった場合は再解析します。初回解析の一時音声は処理後に削除し、元音源は変更しません。
- 面の選択・測定窓・数値・曲一覧・説明を左側に集約し、右側のグラフを縦に大きくしました。ウィンドウの表示領域に合わせて目盛りと曲線を描き直します。
- 面の切り替えや画面を閉じた後に、古い解析応答が表示を上書きしないようにしました。
- Ubuntu標準のFFmpegでも曲単位解析が動作するよう、変換後のステレオ構成を明示しました。
- Windows、Ubuntu 24.04、Linux展開版、Arch Linux、macOS（Apple Silicon / Intel）の配布物と日英の説明・画面画像を更新しました。

グラフは0.5秒間隔で表示します。先頭の測定窓が満たない部分と無音は線が表示されません。面全体の初回解析には長さに応じた処理時間・一時ディスク容量が必要です。デッキや音声出力機器の音量は測定に含みません。目標線は音量統一OFFでも表示します。macOS版はアドホック署名で、Developer ID署名・公証は行っていません。

## English

- Added a selected-track LUFS graph with Short-term (3 seconds) and Momentary (400 ms) windows.
- Added original/processed curves, a target line, integrated LUFS and true peak for the playback range. Cassette entries include their range and fades.
- Added complete A/B side graphs covering leading/trailing silence, gaps, order, ranges and fades, with side switching in the same dialog. Empty sides are supported.
- Whole-side analysis uses the WAV rendering pipeline, retaining continuous measurement windows and gating across track boundaries. Numbered markers and a track legend identify track starts.
- Cached measurements are reused until sources, ranges, order or settings change. Temporary analysis audio is removed; original media is unchanged.
- Moved controls, measurements, track legends and notes to the left and enlarged the graph vertically on the right. Axes and curves adapt to the available window size.
- Stale analysis responses cannot overwrite a newer side selection or a closed dialog.
- Explicitly declared the converted stereo layout so track analysis also works with Ubuntu’s system FFmpeg.
- Updated Windows, Ubuntu 24.04, Linux archive, Arch Linux and macOS (Apple Silicon / Intel) downloads, bilingual instructions and screenshots.

Graphs display measurements at 0.5-second intervals. Incomplete initial windows and silence appear as gaps. First whole-side analysis requires processing time and temporary disk space proportional to side duration. Output-device and deck volume are outside the measurement. The target remains visible when normalization is off. macOS builds use ad-hoc signatures without Developer ID signing or notarization.
