#!/usr/bin/env bash
set -euo pipefail
root=${1:-dist}
test -f "$root/index.html"
# Trunk's local build mutex is not an application asset; build must be finished.
if test -f "$root/.trunk-lock"; then unlink "$root/.trunk-lock"; fi
bash "$(dirname "$0")/check-static.sh" "$root"
