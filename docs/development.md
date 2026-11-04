# Development, builds and validation

[Home](../README.md) · [日本語](development.ja.md)

Normal installation is in the [installation guide](installation.md); operation and specifications are in the [user guide](usage.md). This page covers source builds and packaging.

## Windows / Visual Studio 2026

Install Rust stable MSVC, Visual Studio 2026 with Desktop development with C++, and WebView2. JavaScript tests and URL import need Node.js 22+. Initial dependency downloads need network access.

```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-tools.ps1
cargo build --release --features desktop --bins --locked
```

Setup obtains FFmpeg, FFprobe and yt-dlp under `tools` and checks SHA-256. It does not change system PATH or install Node.js.

Open `CassetteStudio.sln`, choose Debug/Release and x64, then use F5/Ctrl+F5. Visual Studio calls Cargo; a Rust language service is not bundled.

`CassetteStudio.cmd` builds if needed and starts the desktop; `CassetteStudio-Browser.cmd` starts browser mode. Close any existing process using the same library first.

## Ubuntu / Linux

Use Ubuntu 24.04 x64, Rust stable and GUI development libraries.

```sh
sudo apt install build-essential pkg-config libgtk-3-dev libwebkit2gtk-4.1-dev libxdo-dev libdbus-1-dev libssl-dev ffmpeg
cargo run --release --features desktop --bin cassette-studio-desktop --locked
```

`sh scripts/start.sh` also builds/starts the desktop; `sh scripts/start-browser.sh` starts browser mode. URL tools are covered in [installation](installation.md).

WSL helpers `scripts/setup-linux-build.sh` and `scripts/build-linux-wsl.sh` use an isolated Rust environment in `/var/tmp` and copy binaries into `dist/linux-x64`.

## macOS

Install Xcode Command Line Tools, Rust stable and FFmpeg.

```sh
cargo run --release --features desktop --bin cassette-studio-desktop --locked
sh scripts/package-public-macos.sh 0.1.10
```

This builds the current CPU. Apple Silicon and Intel packages use separate Mac runners.

## Arch Linux

Place the release's `PKGBUILD` and `CassetteStudio-0.1.10-Arch-source.tar.gz` in an empty directory, inspect them, then build as a regular user.

```sh
sudo pacman -S --needed base-devel rust pkgconf python
makepkg -si
```

PKGBUILD checks the source SHA-256. `packaging/arch/PKGBUILD.in` is the template; `scripts/prepare-arch-package.py` generates the snapshot and checksummed recipe from the current commit.

## Public release packages

| Platform | Procedure |
| --- | --- |
| Windows | `scripts/package-public-windows.ps1 -Version 0.1.10` |
| All Linux formats | Build native binaries, then `sh scripts/package-all-linux.sh` |
| macOS | `sh scripts/package-public-macos.sh 0.1.10` on each CPU |
| Arch | makepkg above, or `.github/workflows/arch.yml` |

Output goes to `dist/releases/v0.1.10`. License collection needs Python 3 and notices supplied with the Rust toolchain.

`package-windows.ps1` and `package-linux.sh` are separate local helpers bundling external tools. Their output differs from public Windows/traditional Linux downloads; do not mix their usage instructions. Review [redistribution requirements](../THIRD_PARTY_NOTICES.md).

<a name="linux-bundles"></a>

## Linux bundles

Use Ubuntu 24.04 x86_64, native binaries under `dist/linux-x64`, Flatpak, patchelf, mksquashfs, GTK/WebKitGTK development dependencies and FFmpeg.

```sh
flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
flatpak install --user flathub org.gnome.Platform//49
python3 scripts/build-linux-bundles.py flatpak --build-dir /var/tmp/cassette-flatpak-build
python3 scripts/build-linux-bundles.py appimage --build-dir /var/tmp/cassette-appimage-build
```

Use `--resume` for interrupted staging. Downloads are verified against SHA-256 pins in `packaging/linux-bundles/tools.lock.json`; OS versions/notices are recorded. Fonts use app-specific configuration.

`sh scripts/package-all-linux.sh` generates DEB, traditional archive, Flatpak, AppImage and corresponding sources. See [bundled-library sources](linux-bundle-sources.md).

## Runtime configuration

Browser mode starts with `cargo run --release --bin cassette-studio --locked`. Desktop mode embeds the same local engine.

| Variable | Purpose |
| --- | --- |
| `CASSETTE_DATA` | Explicit data-directory override |
| `CASSETTE_PORT` | Native default 3210; Flatpak/AppImage default to a free port |
| `CASSETTE_FFMPEG`, `CASSETTE_FFPROBE`, `CASSETTE_YT_DLP`, `CASSETTE_NODE` | Full tool paths |

Discovery checks explicit overrides, adjacent `tools`, the executable directory, macOS Homebrew paths, development `tools` and PATH. Storage/migration are centralized in [storage and backup](usage.md#storage).

<a name="validation"></a>

## Validation

```sh
cargo fmt --check
cargo test --locked
cargo clippy --features desktop --all-targets --locked -- -D warnings
cargo build --locked
node scripts/test-i18n.cjs
node scripts/test-frontend-startup.cjs
node scripts/test-tape-settings.cjs
node scripts/test-downloads.cjs
node scripts/test-reference-tone.cjs
node scripts/test-entry-fit.cjs
```

Audio checks require FFmpeg/FFprobe in `tools` or configured through environment variables.

| Coverage | Scripts |
| --- | --- |
| Audio/video, silence, fades and ranges | `node scripts/smoke-test.mjs` |
| Automatic creation, empty tapes and exclusion | `node scripts/test-batch.mjs` |
| Loudness, DC and pseudo stereo | `node scripts/test-loudness.mjs`, `node scripts/test-dc.mjs`, `python scripts/test-pseudo-stereo.py` |
| LUFS | `node scripts/test-lufs-graph.mjs`, `node scripts/test-lufs-side-graph.mjs`, `node scripts/test-lufs-graph-ui.cjs` |
| Silence reanalysis, duplicate import and deletion | `node scripts/test-silence-reanalysis.mjs`, `node scripts/test-duplicate-import.mjs`, `node scripts/test-library-delete.mjs` |
| Leading silence / DAT WAV | `node scripts/test-dat-settings.mjs` |
| J-CARD | `node scripts/test-jcard.cjs`, `node scripts/test-jcard-editor.cjs`, `node scripts/test-jcard-api.mjs` |

Tests use synthetic media and isolated libraries under `work`, not the normal library.

After installing a test Flatpak, run `sh scripts/test-linux-bundles.sh --appimage ./dist/linux-bundles/CassetteStudio-0.1.10-x86_64.AppImage` for coexistence/storage/audio checks. `scripts/test-macos-package.py` and `scripts/test-arch-package.py` check packaged apps in their native environments. See [physical-device validation limits](installation.md#coverage).

## Source organization and research

[Source organization](architecture.md) describes UI, button, audio, API and storage responsibilities. For unimplemented Dolby B/C work, see the [research report](dolby-b-c-research.ja.md).
