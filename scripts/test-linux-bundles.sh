#!/bin/sh
set -eu
project=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
# Isolate document-portal mounts from the caller's real graphical session.
export XDG_RUNTIME_DIR=$(mktemp -d /var/tmp/cassette-bundle-runtime.XXXXXX)
chmod 700 "$XDG_RUNTIME_DIR"
exec dbus-run-session -- xvfb-run -a -s '-screen 0 1280x800x24' python3 "$project/scripts/test-linux-bundles.py" "$@"
