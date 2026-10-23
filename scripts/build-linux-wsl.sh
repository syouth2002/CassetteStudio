#!/usr/bin/env sh
# Build on an actual Ubuntu Linux toolchain; keep temporary toolchains out of $HOME.
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
build_dir=${CASSETTE_LINUX_BUILD:-/var/tmp/cassette-studio-linux-build}
mkdir -p "$build_dir"
cp "$project/Cargo.toml" "$project/Cargo.lock" "$build_dir/"
cp -r "$project/src" "$project/web" "$build_dir/"
export RUSTUP_HOME=/var/tmp/cassette-rustup
export CARGO_HOME=/var/tmp/cassette-cargo
export PATH="/var/tmp/cassette-cargo/bin:/usr/local/bin:/usr/bin:/bin"
export CARGO_BUILD_JOBS=2
cd "$build_dir"
cargo build --release --features desktop --bins --locked
cargo test --locked
mkdir -p "$project/dist/linux-x64"
cp target/release/cassette-studio-desktop "$project/dist/linux-x64/CassetteStudio"
cp target/release/cassette-studio "$project/dist/linux-x64/cassette-studio-browser"
