#!/bin/sh
set -eu
: "${XDG_DATA_HOME:=$HOME/.local/share}"
export CASSETTE_DATA="${CASSETTE_DATA:-$XDG_DATA_HOME/cassette-studio-flatpak}"
export CASSETTE_PORT="${CASSETTE_PORT:-0}"
export CASSETTE_FFMPEG="${CASSETTE_FFMPEG:-/app/bin/ffmpeg}"
export CASSETTE_FFPROBE="${CASSETTE_FFPROBE:-/app/bin/ffprobe}"
export CASSETTE_YT_DLP="${CASSETTE_YT_DLP:-/app/bin/yt-dlp}"
export PATH="/app/bin:$PATH"
export FONTCONFIG_FILE="/app/share/cassette-studio/fonts.conf"
export GTK_USE_PORTAL=1
if [ "${CASSETTE_BROWSER:-0}" = 1 ]; then
  exec /app/bin/cassette-studio-browser "$@"
fi
exec /app/bin/cassette-studio-desktop "$@"
