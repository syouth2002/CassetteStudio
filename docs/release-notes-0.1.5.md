# Cassette Studio v0.1.5 / 2026-11-01

## 日本語

- J-CARD編集機能を追加しました。外側／内側のそれぞれに、文字・PNG/JPEG画像・四角形を配置できます。
- ボックスのドラッグ移動、四隅／各辺からのリサイズ、回転、15度刻みの回転、0.5mm吸着、重なり順、中央配置、複製、元に戻す／やり直すに対応しました。
- ゴシック・明朝・等幅の書体、文字サイズ、色、揃え、行間、太字、縦書きを設定できます。リサイズ時は文字サイズを保って折り返します。
- 追加の折り返しパネルとA4／レター用紙に対応しました。裁断ガイドと余白を含めて実寸で1枚に収まる範囲だけ追加できます。初期寸法で幅65mmの追加パネルは+2が最大です。冊子形式は対象外です。
- 表裏2ページの実寸PDF出力に対応しました。裁断・折り曲げの点線とマーキングを必ず付け、表裏を同じ用紙位置へ配置します。SVG出力と、画像を含む編集データの保存・再読込にも対応しました。
- カセットごとの自動保存を追加しました。「閉じる」でも保存完了を待ち、保存失敗時には閉じずに内容を保持します。保存状態は右上へ統一しました。
- 「曲一覧を更新」で最新のミックスリストを取得し、表裏の連動する文字枠を更新します。旧版で連動が外れた既定の曲一覧ボックスも復旧します。「連動する内容」からSIDE A/Bや自由入力を指定できます。
- ボックスの切り取り・コピー・貼り付けとCtrl/Cmd+X/C/Vに対応しました。表裏の間でも書式・回転・連動先を保持します。入力欄内の通常の文字コピー／貼り付けも使えます。
- テンプレート選択・適用、概算のはみ出し警告、重複した保存表示を削除しました。ズームと吸着を左ペインへ、操作説明を右ペインへ移し、編集領域を広げました。
- ネイティブ版の保存ダイアログがPDF・SVG・編集データにも適したファイル種別を表示するようにしました。
- Windows、Ubuntu 24.04、Linux展開版、Arch Linux、macOS（Apple Silicon / Intel）の配布物、日英ドキュメントと画面画像を更新しました。

PDFは日本語の見た目を保つ300dpiの画像方式です。文字を選択できるベクターPDFではありません。印刷時は「実際のサイズ／100%」を指定し、両面の向き・ずれは試し刷りで確認してください。J-CARD編集データは音源・ミックスリストと分けて保存します。macOS版はアドホック署名で、Developer ID署名・公証は行っていません。

## English

- Added a J-CARD editor with independent outside/inside artwork containing text, PNG/JPEG images and rectangles.
- Added drag movement, corner/edge resizing, rotation, 15-degree rotation snapping, 0.5 mm snapping, stacking order, centering, duplication and undo/redo.
- Text supports sans-serif/serif/monospace families, size, color, alignment, line spacing, bold and vertical writing. Resizing preserves font size and reflows text.
- Added fold-out panels and A4/Letter paper selection. Panels must fit on one sheet at actual size including marks and margins. Default dimensions allow at most two extra 65 mm panels. Separate-sheet booklets are not supported.
- Added actual-size two-page PDF output for outside/inside, with mandatory cutting/folding lines and marks and identical page placement. Added SVG output and portable editable files including images.
- Added per-cassette autosave. Close waits for saving; failures retain the open document. Save status appears only at the top right.
- Refresh track lists fetches the latest saved mix and updates linked boxes on both faces. Original legacy track-list boxes with cleared bindings are recognized. Linked content offers side A/B sources and a manual opt-out.
- Added object cut/copy/paste with Ctrl/Cmd+X/C/V, retaining formatting, rotation and links across faces. Native text clipboard operations in input fields still work.
- Removed template selection/application, approximate overflow warnings and duplicate save status. Moved zoom/snapping to the left and handle instructions to the right to enlarge the canvas.
- Native save dialogs now offer suitable file types for PDF, SVG and editable artwork as well as WAV audio.
- Updated Windows, Ubuntu 24.04, Linux archive, Arch Linux and macOS (Apple Silicon / Intel) downloads, bilingual documentation and screenshots.

PDF embeds a 300 dpi image to retain Japanese text appearance, rather than selectable vector text. Print at Actual size / 100% and check duplex orientation/alignment with a test print. Artwork is stored separately from audio and mix data. macOS builds use ad-hoc signatures without Developer ID signing or notarization.
