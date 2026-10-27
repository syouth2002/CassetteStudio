#!/usr/bin/env sh
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
output="$project/dist/linux-x64"
cache="$project/work/linux-tools"
mkdir -p "$output/tools" "$cache"
test -x "$output/CassetteStudio" || { echo 'Build the Linux binaries first.' >&2; exit 1; }
curl --fail --location --silent --show-error https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest -o "$cache/release.json"
release_url=$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(next(a["browser_download_url"] for a in d["assets"] if a["name"]=="yt-dlp_linux"))' "$cache/release.json")
sums_url=$(python3 -c 'import json,sys; d=json.load(open(sys.argv[1])); print(next(a["browser_download_url"] for a in d["assets"] if a["name"]=="SHA2-256SUMS"))' "$cache/release.json")
curl --fail --location --silent --show-error "$release_url" -o "$cache/yt-dlp_linux"
curl --fail --location --silent --show-error "$sums_url" -o "$cache/SHA2-256SUMS"
(cd "$cache"; grep ' yt-dlp_linux$' SHA2-256SUMS | sha256sum --check -)
cp "$cache/yt-dlp_linux" "$output/tools/yt-dlp"
curl --fail --location --silent --show-error https://nodejs.org/dist/index.json -o "$cache/node-releases.json"
node_version=$(python3 -c 'import json,sys; print(next(d["version"] for d in json.load(open(sys.argv[1])) if d["version"].startswith("v24.") and d["lts"]))' "$cache/node-releases.json")
node_archive="node-$node_version-linux-x64.tar.xz"
curl --fail --location --silent --show-error "https://nodejs.org/dist/$node_version/$node_archive" -o "$cache/$node_archive"
curl --fail --location --silent --show-error "https://nodejs.org/dist/$node_version/SHASUMS256.txt" -o "$cache/node-SHASUMS256.txt"
(cd "$cache"; grep " $node_archive\$" node-SHASUMS256.txt | sha256sum --check -)
tar -xJf "$cache/$node_archive" -C "$cache"
cp "$cache/node-$node_version-linux-x64/bin/node" "$output/tools/node"
cp "$cache/node-$node_version-linux-x64/LICENSE" "$output/tools/Node-LICENSE.txt"
curl --fail --location --silent --show-error https://raw.githubusercontent.com/yt-dlp/yt-dlp/master/LICENSE -o "$output/tools/yt-dlp-LICENSE.txt"
chmod 755 "$output/CassetteStudio" "$output/cassette-studio-browser" "$output/tools/node" "$output/tools/yt-dlp"
for document in README.md README.ja.md LICENSE THIRD_PARTY_NOTICES.md; do
    cp "$project/$document" "$output/$document"
done
cp -r "$project/docs" "$output/"
mkdir -p "$output/assets"
cp "$project/assets/cassette-studio-app.png" "$output/assets/"
tar -czf "$project/dist/CassetteStudio-Linux-Ubuntu24-x64.tar.gz" -C "$project/dist" linux-x64
stage=/var/tmp/cassette-studio-deb
mkdir -p "$stage/DEBIAN" "$stage/usr/lib/cassette-studio" "$stage/usr/bin" "$stage/usr/share/applications" "$stage/usr/share/icons/hicolor/512x512/apps"
mkdir -p "$stage/usr/share/doc/cassette-studio"
cp -r "$output/assets" "$output/docs" "$stage/usr/share/doc/cassette-studio/"
cp "$project/LICENSE" "$stage/usr/share/doc/cassette-studio/copyright"
cp "$output/README.md" "$output/README.ja.md" "$output/THIRD_PARTY_NOTICES.md" "$stage/usr/share/doc/cassette-studio/"
cp -r "$output/CassetteStudio" "$output/cassette-studio-browser" "$output/tools" "$stage/usr/lib/cassette-studio/"
cat > "$stage/DEBIAN/control" <<'EOF'
Package: cassette-studio
Version: 0.1.1
Architecture: amd64
Maintainer: Cassette Studio
Depends: libwebkit2gtk-4.1-0, libgtk-3-0t64, libxdo3, libdbus-1-3, ffmpeg, xdg-desktop-portal-gtk
Section: sound
Priority: optional
Description: Cassette playlist editor and recording preparation
 Native desktop window with Japanese and English, audio/video import,
 cassette arrangement, loudness normalization and WAV export.
EOF
cat > "$stage/usr/bin/cassette-studio" <<'EOF'
#!/bin/sh
exec /usr/lib/cassette-studio/CassetteStudio "$@"
EOF
chmod 755 "$stage/usr/bin/cassette-studio"
cat > "$stage/usr/share/applications/cassette-studio.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Cassette Studio
Comment=Prepare and record your cassette mixtapes
Comment[ja]=カセットテープの選曲・録音準備
Exec=cassette-studio
Icon=cassette-studio
Terminal=false
Categories=AudioVideo;Audio;
EOF
cp "$project/assets/cassette-studio-app.png" "$stage/usr/share/icons/hicolor/512x512/apps/cassette-studio.png"
find "$stage" -type d -exec chmod 755 {} +
find "$stage" -type f -exec chmod 644 {} +
chmod 755 "$stage/usr/bin/cassette-studio" "$stage/usr/lib/cassette-studio/CassetteStudio" "$stage/usr/lib/cassette-studio/cassette-studio-browser" "$stage/usr/lib/cassette-studio/tools/node" "$stage/usr/lib/cassette-studio/tools/yt-dlp"
dpkg-deb --root-owner-group --build "$stage" "$project/dist/CassetteStudio-Ubuntu24-amd64.deb"
echo "Ready: $project/dist"
