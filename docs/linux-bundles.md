# Common Linux packages

Flatpak is the primary format for one package across distributions; AppImage is an additional option. Downloads are provided through GitHub Releases; this is not a Flathub app listing. The current target is x86_64.

## Flatpak

    flatpak remote-add --user --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
    flatpak install --user ./CassetteStudio-0.1.9-x86_64.flatpak
    flatpak run io.github.syouth2002.CassetteStudio//local

GTK/WebKitGTK come from GNOME 49. FFmpeg, FFprobe, yt-dlp and Node.js are supplied inside the package. Host tools are unchanged. Initial installation needs network access and runtime disk space. The launcher is named Cassette Studio (Flatpak). This is a local bundle, not an approved Flathub listing.

Display, audio and network access are permitted. Host files are readable, preserving media drag/drop and external-drive imports, but are not globally writable. Save dialogs use desktop portals. Host command execution permission is not granted.

## AppImage

    chmod +x CassetteStudio-0.1.9-x86_64.AppImage
    ./CassetteStudio-0.1.9-x86_64.AppImage

For systems without FUSE:

    APPIMAGE_EXTRACT_AND_RUN=1 ./CassetteStudio-0.1.9-x86_64.AppImage

GTK/WebKitGTK, helpers, WAV playback plugins and media/URL tools are bundled. Base OS libraries and graphics drivers remain relevant. This build targets Ubuntu 24.04 or newer compatible systems with glibc 2.39+. Older distributions are not guaranteed. Prefer Flatpak when a common runtime is the goal.

## Coexistence

Native packages retain their existing storage. Flatpak uses ~/.var/app/io.github.syouth2002.CassetteStudio/data/cassette-studio-flatpak/. AppImage uses ~/.local/share/cassette-studio-appimage/ or the corresponding directory below XDG_DATA_HOME.

New formats select a free internal port. Existing libraries and host FFmpeg are not replaced. Explicit CASSETTE_DATA / CASSETTE_PORT overrides are respected; the existing lock prevents concurrent use of one library.

For a trial copy, close both apps and copy library.json, sources, and optionally jcards into an empty destination. Keep the original. WebView profiles and locks must not be copied; analysis caches can be regenerated. Migration/sharing is never automatic.

## Build and test

Build on Ubuntu 24.04 x86_64 with native Rust binaries in dist/linux-x64. Install Flatpak, patchelf, mksquashfs, GTK/WebKitGTK build dependencies and FFmpeg.

    flatpak install --user flathub org.gnome.Platform//49
    python3 scripts/build-linux-bundles.py flatpak --build-dir /var/tmp/cassette-flatpak-build
    python3 scripts/build-linux-bundles.py appimage --build-dir /var/tmp/cassette-appimage-build

Use --resume for an interrupted staging build. Downloads are verified against tools.lock.json. OS package versions and copyright notices are recorded.

Install the test Flatpak in your user installation, then run:

    sh scripts/test-linux-bundles.sh --appimage ./dist/linux-bundles/CassetteStudio-0.1.9-x86_64.AppImage

The synthetic test checks importing, DC/LUFS, A/B sides, 48 kHz WAV, J-CARD persistence and three simultaneous formats with separate stores/ports. Personal libraries are never used. Current validation is on Ubuntu 24.04 / WSL2. Other distributions, real Wayland sessions, physical audio and drag/drop/save-portal combinations need additional checks; universal Linux compatibility is not claimed.

Noto Sans JP and Noto Sans Symbols 2 are bundled through an application-specific font configuration. Global OS font settings are unchanged.

Corresponding bundled-library sources are supplied as Linux-bundle-sources archives in the same release. They are not needed to run the application.

## Generate every Linux format

After building native binaries on Ubuntu 24.04 and installing the GNOME 49 runtime, run sh scripts/package-all-linux.sh. It produces the DEB, traditional archive, Flatpak, AppImage and corresponding sources under dist/releases. Install the generated Flatpak and run the coexistence test above. See [source and rebuild information](linux-bundle-sources.md).
