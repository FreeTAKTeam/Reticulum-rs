# Contributing to Reticulum-rs

## Goals
- Reliability first.
- Clear protocol boundaries and explicit interfaces.
- Keep proprietary integrations out-of-tree via extension traits.

## Development Setup
- Rust stable (MSRV in crate manifests).
- `cargo install cargo-deny cargo-audit cargo-udeps`

## Local Quality Gates

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo doc --workspace --no-deps
cargo deny check
cargo audit
cargo +nightly udeps --workspace --all-targets
```

## PR Expectations
- Keep changes scoped and reviewable.
- Add or update tests for protocol/runtime behavior changes.
- Mark breaking changes clearly and update docs/ADR.
