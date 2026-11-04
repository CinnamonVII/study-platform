#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
: "${APPIMAGETOOL:?Set APPIMAGETOOL to the verified appimagetool x86_64 executable}"
mkdir -p engine/export/fonts
if [ ! -f engine/export/fonts/DejaVuSans.ttf ]; then
  cp /usr/share/fonts/truetype/dejavu/DejaVuSans.ttf engine/export/fonts/
  cp /usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf engine/export/fonts/
fi
python -m PyInstaller --noconfirm --clean --windowed --name revisions-qt \
  --paths . --collect-submodules engine --collect-submodules apps.desktop \
  --add-data 'fixtures/demo/snapshot-v2.json:fixtures/demo' \
  --add-data 'prompts:prompts' --add-data 'engine/export/fonts:engine/export/fonts' \
  --exclude-module PySide6.QtWebEngineCore --exclude-module PySide6.QtWebEngineWidgets \
  --exclude-module matplotlib --exclude-module pandas --exclude-module numpy \
  apps/desktop/launcher.py
mkdir -p AppDir/usr/bin
cp -a dist/revisions-qt AppDir/usr/bin/
cp packaging/revisions-qt.desktop AppDir/revisions-qt.desktop
cp packaging/revisions.svg AppDir/revisions-qt.svg
cat > AppDir/AppRun <<'RUN'
#!/bin/sh
APPDIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
exec "$APPDIR/usr/bin/revisions-qt/revisions-qt" "$@"
RUN
chmod +x AppDir/AppRun
ARCH=x86_64 "$APPIMAGETOOL" --appimage-extract-and-run AppDir dist/Revisions-0.2.0-x86_64.AppImage
