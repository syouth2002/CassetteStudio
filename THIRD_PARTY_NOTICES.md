# Third-party software / 外部ソフトウェア

[English](README.md) · [日本語](README.ja.md)

## Public release contents / 公開リリースの内容

Public application-only packages include exact Rust crate notices under `licenses/`, Rust standard-library notices, the source of the MPL-2.0 dependency option-ext, and the Microsoft WebView2 SDK loader notices on Windows. The WebView2 loader is matched against SDK 1.0.3800.47. FFmpeg, FFprobe, yt-dlp, Node.js, GTK and WebKitGTK are **not bundled** in these public packages. Install external tools separately. Their licenses are provided by their distributors. The Windows setup helper downloads tools directly from their upstream distributors when the user runs it.

公開用パッケージのlicenses/には、組み込みRustライブラリ・Rust標準ライブラリの表示、MPL-2.0のoption-extのソース、WindowsではMicrosoft WebView2ローダーのライセンス表示を含めます。ローダーはSDK 1.0.3800.47と照合しています。FFmpeg・FFprobe・yt-dlp・Node.js・GTK・WebKitGTKは公開パッケージに同梱しません。外部ツールは別途導入し、配布元のライセンスに従います。Windows付属セットアップは、利用者が実行すると配布元から直接ツールを取得します。

## English

Cassette Studio's own source code and documentation are licensed under [MIT](LICENSE). Dependencies and external tools retain their own licenses; the MIT license does not relicense them. This source repository does not contain external executable files, downloaded tool archives, music, or user libraries.

Rust dependencies and exact versions are recorded in [Cargo.toml](https://github.com/syouth2002/CassetteStudio/blob/v0.1.1/Cargo.toml) and [Cargo.lock](https://github.com/syouth2002/CassetteStudio/blob/v0.1.1/Cargo.lock). Cargo retrieves their source and license files when building. Main dependencies include Axum, Tokio, Serde, serde_json, UUID, tower-http, url, fs2, directories, encoding_rs, Wry, Tao, and rfd. Refer to each crate's own license, including its transitive dependencies, when redistributing a compiled application.

External programs are invoked as separate processes:

| Tool | Upstream information | Distribution notes |
| --- | --- | --- |
| FFmpeg / FFprobe | [FFmpeg legal information](https://ffmpeg.org/legal.html), [Windows build provider](https://www.gyan.dev/ffmpeg/builds/) | License depends on the build configuration. Gyan builds used by the Windows setup script include GPL components. Preserve notices and meet the corresponding source and redistribution requirements for the exact build and its libraries. |
| yt-dlp | [Repository and licensing information](https://github.com/yt-dlp/yt-dlp#license), [license](https://github.com/yt-dlp/yt-dlp/blob/master/LICENSE) | Source and bundled executable licensing can differ because executables include additional components. Review the exact release, its dependency licenses, and notices before redistribution. |
| Node.js | [License and third-party notices](https://github.com/nodejs/node/blob/main/LICENSE) | Preserve the license and bundled third-party notices for the exact version. |
| WebView2 / GTK / WebKitGTK and OS components | Their respective vendors and OS package metadata | Installed runtimes and system libraries retain their own licenses and installation terms. |

The packaging scripts are local build helpers. They do not constitute a complete license audit or a corresponding-source offer. Before uploading a binary bundle, collect license texts, notices, and any required corresponding source for all included components, including Rust dependencies. Use media and URLs only when you have permission to do so; the project's MIT license grants no rights to third-party music or videos.

## 日本語

Cassette Studio自身のソースコードとドキュメントは[MITライセンス](LICENSE)です。依存ライブラリや外部ツールには各自のライセンスが適用され、MITに変更されるわけではありません。このソースリポジトリには外部実行ファイル、ダウンロード済みツール、音源、ユーザーのライブラリを含めません。

Rust依存関係とバージョンは[Cargo.toml](https://github.com/syouth2002/CassetteStudio/blob/v0.1.1/Cargo.toml)と[Cargo.lock](https://github.com/syouth2002/CassetteStudio/blob/v0.1.1/Cargo.lock)に記録しています。ビルド時にCargoがソースやライセンスを取得します。主な依存関係はAxum、Tokio、Serde、serde_json、UUID、tower-http、url、fs2、directories、encoding_rs、Wry、Tao、rfdです。実行ファイルを再配布する際は、間接依存も含めて各ライセンスを確認してください。

- FFmpeg / FFprobe：[公式のライセンス情報](https://ffmpeg.org/legal.html)と[Windowsビルド配布元](https://www.gyan.dev/ffmpeg/builds/)を参照してください。ビルド構成により条件が異なり、Windowsセットアップで使うGyanビルドにはGPLの構成要素があります。実際に配布するバージョンと依存ライブラリの表示・ソース提供などの条件を満たす必要があります。
- yt-dlp：[ライセンス案内](https://github.com/yt-dlp/yt-dlp#license)と[ライセンス本文](https://github.com/yt-dlp/yt-dlp/blob/master/LICENSE)を参照してください。配布実行ファイルには追加の構成要素が含まれるため、ソースと実行ファイルの条件が異なる場合があります。
- Node.js：[ライセンスと第三者の表示](https://github.com/nodejs/node/blob/main/LICENSE)を、配布するバージョンに合わせて保持してください。
- WebView2、GTK、WebKitGTKなどは、それぞれの提供元やOSパッケージの条件に従います。

パッケージ作成スクリプトはローカルビルド用です。ライセンス監査の完了や、対応ソースの提供を保証するものではありません。バイナリ一式を公開する前に、Rustの依存関係も含め、ライセンス本文・表示・必要な対応ソースを揃えてください。音楽や動画の権利は本プロジェクトのMITライセンスには含まれません。
