#!/usr/bin/env bash
set -euo pipefail
if [[ "$(git rev-parse --is-shallow-repository)" == true ]]; then
  git fetch --prune --no-tags --unshallow origin
fi
git fetch --prune --no-tags origin '+refs/heads/*:refs/remotes/origin/*'
default_ref="refs/remotes/origin/${DEFAULT_BRANCH:-main}"
git rev-parse --verify "$default_ref" >/dev/null
printf 'Remote refs fetched successfully. Default branch commit: `%s`\n\n' "$(git rev-parse "$default_ref")"
printf 'Each run uses a disposable checkout. Local working copies are not updated.\n\n'
printf '```text\nahead behind remote-branch\n'
while IFS= read -r ref; do
  [[ "$ref" == refs/remotes/origin/HEAD ]] && continue
  counts=$(git rev-list --left-right --count "$ref...$default_ref")
  printf '%s %s\n' "$counts" "${ref#refs/remotes/origin/}"
done < <(git for-each-ref --format='%(refname)' refs/remotes/origin/)
printf '```\n'
