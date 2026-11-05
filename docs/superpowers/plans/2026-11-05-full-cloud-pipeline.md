# Full private cloud pipeline implementation plan

**Goal:** Run the existing Rust ingestion, change detection, generation, export and accepted-material publication pipeline without an always-on personal machine.

**Architecture:** A separate private repository owns its scheduled workflow, explicit source configuration, immutable originals and coherent SQLite checkpoints. Disposable hosted runners restore that state. Publication attempts must be checkpointed remotely before an external write; failed pushes stop publication. New outputs remain pending review. A native Notion PDF is not claimed from a Typst export.

**Constraints:** No student data or API secrets in the public code repository. No new source classifications invented. Storage choice and API generation credentials remain required user configuration. The Notion integration currently has read-only access; writes require an updated capability.

## Tasks

- [x] Add a checkpoint hook to the Rust publication journal; verify failed checkpoints stop external writes.
- [x] Provide a private-repository hosted workflow and coherent backup script, including failure recovery and private-only execution checks.
- [x] Document exact activation requirements and the limits of automatic PDF export and human review.
- [ ] Review and run focused tests, then publish the code. Activate and verify against real sources only after storage and API configuration are confirmed.

Review found a runner-loss window during generation. The engine now checkpoints each completed document before starting the next paid call. Native Notion PDF layout and publication-format updates remain a prerequisite for declaring generated fiches conformant.
