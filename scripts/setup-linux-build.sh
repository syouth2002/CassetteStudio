#!/usr/bin/env sh
set -eu
export RUSTUP_HOME=/var/tmp/cassette-rustup
export CARGO_HOME=/var/tmp/cassette-cargo
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /var/tmp/cassette-rustup-init.sh
sh /var/tmp/cassette-rustup-init.sh -y --profile minimal --no-modify-path
