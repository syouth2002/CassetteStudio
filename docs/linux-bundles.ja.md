# Linux共通パッケージ

ディストリビューション別のビルドを減らすため、Flatpakを主形式、AppImageを補助形式として配布します。GitHub Releasesから取得できます。Flathubのアプリ一覧には登録していません。現在の生成対象はx86_64です。

## Flatpak

同じ.flatpakを、Flatpakが利用できるディストリビューションで使用します。GTK・WebKitGTKは共通のGNOME 49ランタイムを使用します。OS側のFFmpegを変更せず、アプリ内のFFmpeg・FFprobe・yt-dlp・Node.jsを使います。

    flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
    flatpak install --user ./CassetteStudio-0.1.7-x86_64.flatpak
    flatpak run io.github.syouth2002.CassetteStudio//local

初回はランタイム取得にネット接続と追加容量が必要です。アプリ一覧では「Cassette Studio (Flatpak)」です。Flathub登録済みアプリではなく、手元で作成したバンドルのインストールです。

音声出力・画面・ネットワークを許可します。音源のD&Dと外付けドライブからの取り込みを維持するため、ホスト側ファイルを読み取り専用で参照します。保存先選択にはデスクトップポータルを使います。ホスト全体への書き込み権限やホストコマンド実行権限は付けません。

## AppImage

    chmod +x CassetteStudio-0.1.7-x86_64.AppImage
    ./CassetteStudio-0.1.7-x86_64.AppImage

GTK・WebKitGTK、補助プロセス、WAV再生用プラグイン、音声／URLツールを含めます。基礎ライブラリやグラフィックドライバーまでOSから独立する形式ではありません。現在のビルドはUbuntu 24.04を基準とし、glibc 2.39以降が必要です。古いディストリビューションへの対応は保証しません。共通実行環境を優先する場合はFlatpakを選んでください。

FUSEが使えない環境では展開実行できます。

    APPIMAGE_EXTRACT_AND_RUN=1 ./CassetteStudio-0.1.7-x86_64.AppImage

## 既存版との併用

| 形式 | 既定のライブラリ保存先 |
| --- | --- |
| 通常版 | 従来の保存先を維持 |
| Flatpak | ~/.var/app/io.github.syouth2002.CassetteStudio/data/cassette-studio-flatpak/ |
| AppImage | ~/.local/share/cassette-studio-appimage/（XDG_DATA_HOME設定時はその配下） |

新形式は空いている内部通信ポートを選びます。通常版のライブラリ・FFmpeg・実行ファイルを置換せず、別ライブラリで同時起動できます。CASSETTE_DATA／CASSETTE_PORTの明示指定は優先します。Flatpakでホスト側の任意の保存先を指定する場合は、そのフォルダーへの書き込み許可も必要です。同じライブラリの同時使用は既存のロックで拒否します。

既存ライブラリを試す場合は両アプリを終了し、空のコピー先へlibrary.json・sources・必要ならjcardsをコピーしてください。元の保存先は残します。webviewやロック、解析キャッシュはコピー不要です。自動移行・共有は行いません。

## ビルドと検証

Ubuntu 24.04 x86_64でRustの通常Linux版を先に生成し、dist/linux-x64へ配置します。Flatpak・patchelf・mksquashfs・GTK/WebKitGTKの開発環境・FFmpegが必要です。

    flatpak install --user flathub org.gnome.Platform//49
    python3 scripts/build-linux-bundles.py flatpak --build-dir /var/tmp/cassette-flatpak-build
    python3 scripts/build-linux-bundles.py appimage --build-dir /var/tmp/cassette-appimage-build

途中のステージを再利用する場合は--resumeを指定します。ダウンロードはtools.lock.jsonのSHA-256で検証します。OS由来の依存ライブラリについてはパッケージ名・版・著作権表示を収録します。

テスト用Flatpakをユーザー領域へインストール後、D-Bus／仮想画面内で次を実行します。合成音源だけを使い、個人のライブラリは使用しません。

    sh scripts/test-linux-bundles.sh --appimage ./dist/linux-bundles/CassetteStudio-0.1.7-x86_64.AppImage

取り込み、DC/LUFS、A/B面、48kHz WAV、J-CARD保存、3形式の同時起動・保存先分離を検証します。現在の検証環境はUbuntu 24.04／WSL2です。他ディストリビューションの実機、Wayland、物理音声機器、D&D・保存ポータルの組み合わせは別途確認が必要です。「全Linux環境で動く」という保証ではありません。

日本語・操作記号用にNoto Sans JP／Noto Sans Symbols 2を同梱し、アプリ専用のフォント設定で参照します。OS全体のフォント設定は変更しません。

同梱ライブラリの対応ソースは、同じリリース内のLinux-bundle-sourcesアーカイブで提供します。通常利用には不要です。

## 全Linux形式の一括生成

Ubuntu 24.04で通常版をビルドし、GNOME 49ランタイムを導入した後、sh scripts/package-all-linux.shを実行します。DEB・従来版・Flatpak・AppImageと対応ソースをdist/releases内に生成します。Flatpakの導入と同時起動テストは上記の手順で行ってください。ソースの一覧・再ビルドについては[対応ソースの説明](linux-bundle-sources.md)を参照してください。
