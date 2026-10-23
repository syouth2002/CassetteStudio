#!/usr/bin/env sh
set -eu
cd "$(dirname "$0")/.."
cargo build --release --bin cassette-studio --locked
echo "Open http://127.0.0.1:${CASSETTE_PORT:-3210}/ in your browser."
exec ./target/release/cassette-studio
