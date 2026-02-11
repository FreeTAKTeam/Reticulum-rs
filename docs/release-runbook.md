# Release Runbook

## Preconditions
- Workspace CI is green.
- Cross-repo LXMF compatibility job is green.
- ADR/docs updated for architectural changes.

## Steps
1. Run `cargo fmt`, `clippy`, `test`, `doc`, `deny`, and `audit`.
2. Validate daemon + core behavior on clean checkout.
3. Create signed release tag (`git tag -s`).
4. Publish release and attach notes.

## Checklist
- [ ] Version bump merged
- [ ] Compatibility matrix updated
- [ ] Signed tag pushed
- [ ] Roll-forward/rollback notes recorded
