#!/bin/sh
# Build a native-architecture app; external audio tools remain separate.
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
version=${1:-0.1.2}
case $(uname -m) in arm64) arch=arm64; target=aarch64-apple-darwin;; x86_64) arch=x64; target=x86_64-apple-darwin;; *) exit 1;; esac
release="$project/dist/releases/v$version"
stage="$release/macos-$arch"
app="$stage/Cassette Studio.app"
if [ -n "${CASSETTE_PREBUILT_ARCHIVE:-}" ]; then
    test ! -e "$stage"
    ditto -x -k "$CASSETTE_PREBUILT_ARCHIVE" "$release"
    test -x "$app/Contents/MacOS/CassetteStudio"
else
test ! -e "$stage" || { echo "Stage already exists: $stage" >&2; exit 1; }
cd "$project"
cargo build --release --features desktop --bins --locked
cargo test --locked
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources" "$stage/docs"
cp target/release/cassette-studio-desktop "$app/Contents/MacOS/CassetteStudio"
cp target/release/cassette-studio "$stage/cassette-studio-browser"
cp README.md README.ja.md LICENSE THIRD_PARTY_NOTICES.md "$stage/"
cp -r docs/screenshots "$stage/docs/"
cp docs/dolby-b-c-research.ja.md "$stage/docs/"
python3 scripts/collect-licenses.py --target "$target" --output "$stage/licenses"
iconset="$release/cassette-$arch.iconset"
mkdir -p "$iconset"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" assets/cassette-studio-app.png --out "$iconset/icon_${size}x${size}.png" >/dev/null
    twice=$((size * 2))
    sips -z "$twice" "$twice" assets/cassette-studio-app.png --out "$iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$iconset" -o "$app/Contents/Resources/CassetteStudio.icns"
fi
python3 - "$app/Contents/Info.plist" "$version" <<'PY'
import plistlib,sys
with open(sys.argv[1],'wb') as output:
    plistlib.dump({'CFBundleName':'Cassette Studio','CFBundleDisplayName':'Cassette Studio',
      'CFBundleIdentifier':'com.syouth2002.CassetteStudio','CFBundleExecutable':'CassetteStudio',
      'CFBundlePackageType':'APPL','CFBundleShortVersionString':sys.argv[2],
      'CFBundleVersion':sys.argv[2],'CFBundleIconFile':'CassetteStudio.icns',
      'LSMinimumSystemVersion':'15.0','NSHighResolutionCapable':True,
      'NSAppTransportSecurity':{'NSAllowsLocalNetworking':True,
        'NSExceptionDomains':{'127.0.0.1':{'NSExceptionAllowsInsecureHTTPLoads':True}}}},output)
PY
cat > "$stage/SetupAudioTools.command" <<'SH'
#!/bin/sh
set -eu
for candidate in /opt/homebrew/bin/brew /usr/local/bin/brew; do
    if [ -x "$candidate" ]; then exec "$candidate" install ffmpeg yt-dlp node; fi
done
echo 'Install Homebrew from https://brew.sh, then run this helper again.'
echo 'Homebrewを導入してから、このファイルを再度実行してください。'
read -r answer
exit 1
SH
chmod 755 "$app/Contents/MacOS/CassetteStudio" "$stage/cassette-studio-browser" "$stage/SetupAudioTools.command"
codesign --force --sign - "$app"
codesign --verify --deep --strict "$app"
ditto -c -k --sequesterRsrc --keepParent "$stage" "$release/CassetteStudio-$version-macOS-$arch.zip"
echo "Ready: $stage"
