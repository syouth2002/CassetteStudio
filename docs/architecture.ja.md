# ソースコードの役割と変更場所

## Rust側

| 場所 | 役割 |
| --- | --- |
| `src/lib.rs` | アプリ外部へ公開する起動API。専用ウィンドウ版とブラウザ版の共通入口。 |
| `src/server/mod.rs` | ローカルサーバーの起動・終了、ルートの組み合わせ。 |
| `src/server/assets.rs` | HTML・CSS・JavaScript・翻訳・アイコンの埋め込み配信。配信対象を明示的に限定。 |
| `src/server/api.rs` | HTTPの入力検証と、ライブラリ・配置・解析・書き出し操作の受け付け。 |
| `src/server/state.rs` | ライブラリの所有・読み書き・排他制御、旧タグの移行、解析結果の更新、処理数の制限。 |
| `src/server/imports.rs` | 元ファイルのSHA-256照合、旧音源の識別情報の更新、未登録一時ファイルの後始末。 |
| `src/server/security.rs` | 別のWebサイトからの変更要求を拒否する処理。 |
| `src/audio/mod.rs` | 音声機能の共通入口。呼び出し側は個別の実装ファイルに依存しない。 |
| `src/audio/tools.rs` | FFmpegなどの探索と外部プロセス実行。 |
| `src/audio/metadata.rs` | 日本語タグの文字コード補正と関連テスト。 |
| `src/audio/import.rs` | 音源の情報取得、ローカル／URL音源の取り込み。 |
| `src/audio/analysis.rs` | 前後の無音・ラウドネス・左右のDCオフセットの測定。 |
| `src/audio/processing.rs` | DC補正フィルター、一定のゲイン補正とピーク保護、試聴キャッシュ名。 |
| `src/audio/render.rs` | 曲間・フェードを反映したWAV組み立てと、曲全体の試聴音声生成。 |
| `src/audio/tests.rs` | ゲイン、DC、キャッシュ、無音範囲、旧データの単体テスト。 |
| `src/model.rs` | 保存データの型、既定値、設定値の検証。 |
| `src/planner.rs` | カセットの自動配置、重複除外、面の長さの検証。 |
| `src/bin/desktop.rs` / `src/main.rs` | 専用ウィンドウ／ブラウザ用エンジンの起動。 |

試聴とWAV生成は引き続き同じDC・ゲイン計算を使います。ファイル分割に伴って音声フィルターの順番や計算式、保存データの形式は変更していません。

## 画面側

| 場所 | 役割 |
| --- | --- |
| `web/index.html` | 画面と各ダイアログの構造、スクリプトの読み込み順。 |
| `web/style.css` | レイアウト、部品の見た目、狭い画面での配置。 |
| `web/app.js` | 操作の登録と最初のライブラリ読み込みだけを行う起動処理。 |
| `web/js/core.js` | DOM参照、HTMLのエスケープ、時刻・dBの表示用整形。 |
| `web/js/state.js` | 選択中のライブラリ、設定、再生状態、保存待ち処理などの共有状態。 |
| `web/js/api.js` | HTTP呼び出しとコレクション保存の順序制御。 |
| `web/js/dialogs.js` | 通知、処理中の表示、名前入力・確認ダイアログ。 |
| `web/js/view.js` | 左ペイン、曲一覧、面の残り時間、選択・再生中表示の描画。 |
| `web/js/library.js` | ライブラリ・コレクションのデータ操作、音源の追加。 |
| `web/js/cassette.js` | 面の容量に合わせる計算、自動配置の要求。 |
| `web/js/settings.js` | 設定データと入力欄の相互変換、設定の適用。 |
| `web/js/export.js` | 書き出し要求、カセット別の生成済み音声と保存リンク。 |
| `web/js/audio/player.js` | 再生・シーク・曲間・曲送り・停止の制御。 |
| `web/js/audio/reference-tone.js` | 基準音のWAV生成と再生・停止。 |
| `web/js/audio/loudness.js` | 解析済みデータから表示用の補正量を計算。実際の補正はRust側。 |
| `web/js/buttons/` | 操作イベントの登録。ライブラリ、解析、範囲編集、設定、再生、共通処理に分割。 |
| `web/i18n.js` / `web/en.json` | 言語の選択、翻訳、表示言語の保存。 |

画面側はビルドツール不要の通常のスクリプトを維持しています。ES Modulesによる独立したスコープではなく、共有状態は`state.js`で宣言し、機能ファイルから参照します。HTMLに列挙した順序で定義を読み込み、最後の`app.js`でイベントを一度だけ登録します。言語変更や再描画では登録し直しません。

## 変更するときの目安

- 表示内容・一覧の見せ方: `view.js`。色や配置は`style.css`。
- ボタンや入力の動作: `buttons/`内の該当ファイル。通信の共通処理は`api.js`。
- フェード／容量合わせ: 画面編集は`cassette.js`・`buttons/entries.js`、自動選曲はRustの`planner.rs`、実際の音声生成は`audio/render.rs`。
- DCや音量補正: 測定は`audio/analysis.rs`、補正式は`audio/processing.rs`。
- 新しいJavaScriptファイル: `index.html`の読み込み順と`server/assets.rs`の埋め込み対象を両方更新。

## 確認方法

`cargo test --locked`と、README記載の音源取り込み・音量・DCの結合テストを使用します。

`node scripts/test-frontend-startup.cjs`は、起動したエンジンが配信するHTMLから実際の読み込み順で全スクリプトを取得し、テスト用DOMで起動・ボタン登録・通知・基準音・自動設定・言語切替を確認します。ブラウザの見た目や実音声出力を検証するテストではありません。

既存の画面側テストも、関数を一行の文字列として抜き出す方式から、役割別ファイルを直接読み込む方式へ変更しています。整形やファイル移動でテストが壊れにくい構成です。
