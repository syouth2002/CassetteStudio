#!/bin/sh
# Build native, Flatpak and AppImage release assets in one Ubuntu 24.04 session.
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$project/Cargo.toml" | head -n 1)
release="$project/dist/releases/v$version"
build=$(mktemp -d /var/tmp/cassette-all-linux.XXXXXX)
sh "$project/scripts/package-public-linux.sh" "$version"
python3 "$project/scripts/build-linux-bundles.py" flatpak --build-dir "$build/flatpak" --output "$release"
python3 "$project/scripts/build-linux-bundles.py" appimage --build-dir "$build/appimage" --output "$release"
python3 "$project/scripts/collect-linux-bundle-sources.py" --appdir "$build/appimage/CassetteStudio.AppDir" --output /var/tmp/cassette-bundle-sources
python3 "$project/scripts/package-linux-bundle-sources.py" --sources /var/tmp/cassette-bundle-sources --output "$release"
echo "All Linux assets ready: $release (build workspace: $build)"
