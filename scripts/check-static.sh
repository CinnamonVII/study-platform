#!/usr/bin/env bash
set -euo pipefail
root=${1:-dist}
test -f "$root/index.html"
test -f "$root/manifest.webmanifest"
while IFS= read -r -d '' path; do
  case "$path" in
    *.html|*.js|*.wasm|*.png|*.webmanifest) ;;
    *) echo "Artifact statique inattendu : $path" >&2; exit 1;;
  esac
done < <(find "$root" -type f -print0)
if find "$root" -type f \( -name '.env*' -o -name '*.pdf' -o -name '*.sqlite*' -o -name 'snapshot.json' \) | read -r _; then
  echo 'Contenu privé interdit dans les artifacts Pages' >&2; exit 1
fi
