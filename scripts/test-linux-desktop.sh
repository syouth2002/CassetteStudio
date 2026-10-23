#!/usr/bin/env sh
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
test_dir=/var/tmp/cassette-studio-native-test
mkdir -p "$test_dir"
export CASSETTE_DATA="$test_dir/data"
export CASSETTE_PORT=3215
export GDK_BACKEND=x11
xvfb-run -a "$project/dist/linux-x64/CassetteStudio" > "$test_dir/window.log" 2>&1 &
launcher=$!
trap 'pkill -TERM -P "$launcher" 2>/dev/null || true; kill "$launcher" 2>/dev/null || true' EXIT
python3 "$project/scripts/test-linux-desktop.py" "$project" "$test_dir"
