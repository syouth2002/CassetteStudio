#!/bin/sh
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
version=${1:-0.1.0}
release="$project/dist/releases/v$version"
stage="$release/linux-x64"
test ! -e "$stage" || { echo "Stage already exists: $stage" >&2; exit 1; }
mkdir -p "$stage/docs" "$stage/assets"
cp "$project/assets/cassette-studio-app.png" "$stage/assets/"
cp "$project/dist/linux-x64/CassetteStudio" "$project/dist/linux-x64/cassette-studio-browser" "$stage/"
for file in README.md README.ja.md LICENSE THIRD_PARTY_NOTICES.md; do cp "$project/$file" "$stage/$file"; done
cp -r "$project/docs/screenshots" "$stage/docs/"
python3 "$project/scripts/collect-licenses.py" --target x86_64-unknown-linux-gnu --output "$stage/licenses"
chmod 755 "$stage/CassetteStudio" "$stage/cassette-studio-browser"
tar -czf "$release/CassetteStudio-$version-Linux-Ubuntu24-x64.tar.gz" -C "$release" linux-x64
deb=$(mktemp -d /var/tmp/cassette-public-deb.XXXXXX)
mkdir -p "$deb/DEBIAN" "$deb/usr/lib/cassette-studio" "$deb/usr/bin" "$deb/usr/share/applications" "$deb/usr/share/icons/hicolor/512x512/apps" "$deb/usr/share/doc/cassette-studio"
cp "$stage/CassetteStudio" "$stage/cassette-studio-browser" "$deb/usr/lib/cassette-studio/"
cp -r "$stage/licenses" "$stage/docs" "$stage/assets" "$deb/usr/share/doc/cassette-studio/"
cp "$stage/LICENSE" "$deb/usr/share/doc/cassette-studio/copyright"
cp "$stage/README.md" "$stage/README.ja.md" "$stage/THIRD_PARTY_NOTICES.md" "$deb/usr/share/doc/cassette-studio/"
cat > "$deb/DEBIAN/control" <<EOF
Package: cassette-studio
Version: $version
Architecture: amd64
Maintainer: syouth2002
Depends: libwebkit2gtk-4.1-0, libgtk-3-0t64, libxdo3, libdbus-1-3, ffmpeg, xdg-desktop-portal-gtk
Suggests: yt-dlp, nodejs
Section: sound
Priority: optional
Description: Cassette mixtape editor and recording preparation
 Native desktop application with Japanese and English interfaces.
EOF
printf '#!/bin/sh\nexec /usr/lib/cassette-studio/CassetteStudio "$@"\n' > "$deb/usr/bin/cassette-studio"
cat > "$deb/usr/share/applications/cassette-studio.desktop" <<'EOF'
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
cp "$project/assets/cassette-studio-app.png" "$deb/usr/share/icons/hicolor/512x512/apps/cassette-studio.png"
find "$deb" -type d -exec chmod 755 {} +
find "$deb" -type f -exec chmod 644 {} +
chmod 755 "$deb/usr/bin/cassette-studio" "$deb/usr/lib/cassette-studio/CassetteStudio" "$deb/usr/lib/cassette-studio/cassette-studio-browser"
dpkg-deb --root-owner-group --build "$deb" "$release/CassetteStudio-$version-Ubuntu24-amd64.deb"
echo "Ready: $release"
