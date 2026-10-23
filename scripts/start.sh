#!/usr/bin/env sh
set -eu
cd "$(dirname "$0")/.."
cargo build --release --features desktop --bin cassette-studio-desktop --locked
exec ./target/release/cassette-studio-desktop
