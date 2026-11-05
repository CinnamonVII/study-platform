# Stateless automation implementation plan

**Goal:** Automate only operations that can run on disposable GitHub-hosted runners.

**Architecture:** Hourly fetches report current remote branch status. Daily Notion checks read metadata for an explicit list of pages, without content exports, checkpoints, or writes. The existing persistent pipeline remains manual.

**Tech stack:** GitHub Actions, Bash/Git, Node built-ins for a small Notion API check; the application remains Rust.

**Spec:** User request: “only automate what doesnt need persistent storage”.

## Constraints and review focus

- No local service, runner, database, cache, or private content artifacts.
- Never treat a skipped Notion check as successful access.
- Never log tokens, page titles, IDs, attachment URLs, or response bodies.
- Invalid configuration and partial/revoked page access must fail clearly.
- Fetch reports remote refs; it does not update the user's working tree.

## Tasks

- [x] Add an hourly hosted fetch workflow and branch comparison script; verify against the actual remote.
- [x] Add a daily hosted Notion metadata check with explicit configuration; test missing/invalid configuration and partial API failures using small offline tests.
- [x] Remove the persistent pipeline's schedule; update deployment and task documentation to reflect the new boundary.
- [ ] Review the diff, publish the workflows, dispatch hosted verification runs, and inspect their actual conclusions. Cancel obsolete queued persistent runs.
