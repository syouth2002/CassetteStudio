# Cassette Studio v0.1.4 / 2026-10-29

## 日本語

- 片面の長さを「分・秒」で指定する形式へ変更し、長さの補正と途中再生時の巻き戻し設定を削除しました。既存カセットの実効時間は保持します。
- 無音の最小継続時間を0.1秒刻み、初期値0.5秒に変更し、簡単な説明を追加しました。録音設定内の項目配置も整理しました。
- 無音の再解析を選択曲だけに限定し、既存カセット・ミックスリストの再生範囲にも反映します。途中の長い無音や、その後に再開する音を保持する判定へ改善しました。
- 同一内容のファイルの重複登録を防止します。名前が違っても同じ内容なら登録済みの曲を再利用します。既存の重複曲は自動統合しません。
- 「すべての曲」で選択した曲を削除できない問題を修正しました。確認後、各プレイリスト・カセット内の参照も除去します。元ファイルと取り込み済みの音源ファイルは保持します。
- 「デッキ録音用に準備」に、5秒後の開始に加えて即時の「再生開始」を追加しました。
- ファイル・URL追加、カセットの操作ボタンを整理し、見出しとA/B面表示をコンパクトにしました。スクロール時も面切替・編集ボタン・曲一覧の列見出しを上部に固定します。
- オプション内の名称を「選択曲の音量・DC解析」に変更しました。未選択時に表示中の曲を解析する従来の動作は継続します。無音の再解析は未選択なら実行しません。
- Windows、Ubuntu 24.04、Linux展開版、Arch Linux、macOS（Apple Silicon / Intel）の配布物、日英ドキュメントとスクリーンショットを更新しました。

再解析で曲が長くなりテープ容量を超えた場合は警告します。曲を自動で削除せず、曲順も保持します。再解析後のWAVは再生成してください。macOS版はアドホック署名で、Developer ID署名・公証は行っていません。物理デッキでの録音は検証対象外です。

## English

- Enter per-side tape capacity in minutes and seconds. Removed length correction and continuation rewind; existing effective cassette capacities are retained.
- Minimum silence duration now uses 0.1-second steps with a 0.5-second default. Added a short explanation and reorganized recording settings.
- Reanalyze silence only for selected tracks, including sources already used by cassettes. Existing mix ranges follow revised boundaries. Internal pauses and audio returning after them are preserved.
- Prevent duplicate file registrations by comparing SHA-256 content hashes, including renamed copies. Existing duplicate registrations are not automatically merged.
- Fixed disabled Remove selected in All tracks. Confirmed deletion also removes references from playlists and both cassette sides, while retaining original and imported source files.
- Added immediate playback to the deck recording dialog alongside the five-second countdown.
- Reorganized import and cassette controls, reduced heading and A/B overview sizes, and kept side tabs, editing controls and column headings visible while scrolling.
- Renamed the option to Analyze loudness / DC in selected tracks. Its existing fallback to displayed tracks when none are selected is unchanged; silence reanalysis requires an explicit selection.
- Updated Windows, Ubuntu 24.04, Linux archive, Arch Linux and macOS (Apple Silicon / Intel) downloads, bilingual documentation and screenshots.

If reanalysis increases a mix beyond tape capacity, a warning is shown without deleting tracks or changing their order. Regenerate WAV files after analysis. macOS builds use ad-hoc signatures without Developer ID signing or notarization. Physical deck recording is not covered by automated checks.
