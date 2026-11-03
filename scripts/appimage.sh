#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${LINUXDEPLOY:?Indiquez le chemin du binaire linuxdeploy officiel}"
cargo build --locked --release -p study-ui
mkdir -p AppDir
"$LINUXDEPLOY" --appdir AppDir --executable target/release/study-ui --desktop-file packaging/revisions.desktop --icon-file packaging/revisions.svg --output appimage
