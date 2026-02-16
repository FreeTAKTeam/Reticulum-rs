## Summary

## Type
- [ ] breaking
- [ ] refactor
- [ ] reliability
- [ ] chore/governance

## Validation
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace --all-features`

Optional full-target pass (slower):

- [ ] `cargo test --workspace --all-targets --all-features`
- [ ] `cargo doc --workspace --no-deps`
- [ ] `cargo deny check`
- [ ] `cargo audit`

## Protocol/Compatibility
- [ ] Updated compatibility matrix / ADR (if needed)
