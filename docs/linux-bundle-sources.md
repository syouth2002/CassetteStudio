# Bundled Linux library sources / Linux同梱ライブラリの対応ソース

The optional `Linux-bundle-sources.tar.gz` release asset contains the exact Ubuntu source packages for libraries shipped in the AppImage and Flatpak tool payload. `ubuntu-sources.json` records package versions, original source URLs and SHA-256 hashes. Original `.dsc` manifests include the upstream archive and distribution patches. Use `dpkg-source -x package.dsc` on Ubuntu to unpack each source package, then its `debian/rules` / `debian/control` instructions to build it. GNOME runtime sources are maintained separately by the GNOME/Flathub runtime project, not redistributed inside our app bundle.

`build/packaging/linux-bundles/sources.lock.json` identifies additional upstream sources: yt-dlp, EJS, Mutagen, certifi, curl_cffi, curl-impersonate and its libidn2 source, plus the CentOS 7 Readline 6.2 source/patches used by the bundled yt-dlp runtime. The libidn2 archive includes the configured libunistring code. These upstream source trees contain their own licenses and build instructions; the curl-impersonate tree includes the static dependency build scripts. The original yt-dlp third-party notices and source contact are preserved in each binary package. Source notices do not change the license of the Cassette Studio application.

The build scripts and pinned tool downloads are included under `build/`; see the [Linux build instructions](development.md#linux-bundles) for native Rust binaries. The app's own MIT source is available from the release tag. For inspecting or modifying LGPL components, AppImage supports `--appimage-extract` and Flatpak has normal development/override facilities. Keep the private FFmpeg library path in the tool wrapper separate from the GUI library path.

通常利用には不要なソース用アーカイブです。同梱ライブラリのUbuntuソース・配布元のパッチ・追加ツールのソースを収録し、一覧とSHA-256を付けています。Ubuntuの各ソースは `.dsc` を `dpkg-source -x` で展開し、同梱のビルド手順を参照できます。GNOMEランタイムのソースはGNOME／Flathub側で管理され、アプリ本体には含めていません。アプリ自身のMITソースは同じリリースタグから取得できます。

ビルド手順は[開発者向けガイド](development.ja.md#linux-bundles)にまとめています。
