# Cassette Studio v0.2.0 / 2026-11-14

## 日本語 — v0.1.10からの変更

- Windows・Linux・macOSの専用ウィンドウとブラウザの画面を、共通のRust UI（egui／eframe・WebAssembly）へ移行しました。FFmpegによる取り込み・解析・WAV生成と、既存ライブラリ・J-CARDの保存形式は維持しています。
- 曲の操作を右クリックメニューへまとめ、状態欄を簡潔にしました。「プロパティ」では取得できる音源の形式・コーデック・サンプリングレート・ビット深度・タグと解析結果をまとめて確認できます。モノラル曲は「音量調整・疑似ステレオ」から調整でき、強さの説明も表示します。
- 曲順のドラッグに移動中の曲・挿入線・上下端の自動スクロールを追加しました。画面や入力欄の余白、通知、ダイアログの保存・閉じる操作を整理しています。
- 通常再生の曲送りを元のリストと再生位置に基づく方式へ統一しました。カセット再生の準備中はダイアログを表示し、完了すると自動で閉じて再生します。取り消し後の遅い応答は再生しません。
- WAV保存とデッキ録音を「WAV出力・デッキ録音」に統合しました。変更のないWAVは再利用し、A/B面の保存リンクをダイアログ内に表示します。録音専用プレーヤーは通常再生から独立し、面選択・一時停止・停止・シーク・音量・5秒後開始に対応します。閉じると録音用再生と開始待ちを停止します。
- J-CARDに編集表示の90度回転、カーソル中心ズーム、中央ボタン／Spaceドラッグによる移動、移動範囲制限、右クリック編集を追加しました。折り目間の中央揃えと、重なった背面要素の編集に対応します。編集表示の回転は印刷方向を変えません。
- LUFSグラフの元データを赤い点線で表示し、解析中の案内と解析済みデータの再利用を改善しました。手動の曲追加でもA/B面に配置します。
- UI・再生状態・イベント処理を役割ごとのモジュールへ整理しました。日英ガイドと画面画像、各OSの梱包・依存関係を新UIに更新しました。

## English — Changes since v0.1.10

- Migrated native Windows/Linux/macOS windows and the browser to a shared Rust egui/eframe and WebAssembly UI, retaining FFmpeg processing and existing library/J-CARD formats.
- Consolidated track commands into context menus and compact status badges. Properties combines available media specifications, tags and analysis; mono sources use a single loudness/pseudo-stereo dialog with strength explanations.
- Added drag previews, insertion lines and edge auto-scrolling, and refined spacing, inputs, notifications and dialog actions.
- Unified track navigation around playback-list snapshots. Cassette playback shows cancellable preparation and starts automatically when audio is ready.
- Combined WAV downloads and dedicated recording in one dialog. Unchanged WAVs are reused. The independent recording player offers A/B selection, pause/stop/seek/volume and delayed start; closing stops playback and cancels its countdown.
- Added J-CARD quarter-turn view rotation, cursor-centered zoom, bounded panning and context editing. Centering uses fold regions; selected rear objects remain editable through overlaps. View rotation does not change printing orientation.
- Improved LUFS cache reuse and loading feedback, with original audio shown as a red dashed line. Manual additions populate both cassette sides.
- Split UI orchestration by responsibility and refreshed bilingual guides, screenshots and platform packaging.

AppImage targets Ubuntu 24.04-compatible glibc 2.39+ systems. Flatpak uses GNOME 49. macOS packages use ad-hoc signing without Developer ID notarization. Synthetic/virtual-display tests do not validate physical decks, every desktop portal or printer alignment.
