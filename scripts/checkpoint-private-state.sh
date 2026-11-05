#!/usr/bin/env bash
# Executed from the private data repository, including before publication writes.
set -euo pipefail
[[ "${STUDY_STATE:-}" == state ]] || { echo 'STUDY_STATE must be relative path state' >&2; exit 1; }
[[ "${STUDY_PRIVATE_REPOSITORY:-}" == true ]] || { echo 'Private repository verification required' >&2; exit 1; }
python3 - <<'PY'
import os, sqlite3
from pathlib import Path
root = Path('state')
if (root / 'study.sqlite').exists():
    temporary = root / 'checkpoint.tmp'
    with sqlite3.connect(root / 'study.sqlite') as source:
        with sqlite3.connect(temporary) as destination:
            source.backup(destination)
    os.replace(temporary, root / 'checkpoint.sqlite')
PY
git add -- state
if ! git diff --cached --quiet; then
  git commit -m 'study: checkpoint private pipeline state'
fi
# Push even after an earlier failed push left an otherwise clean index.
git push origin HEAD:main
