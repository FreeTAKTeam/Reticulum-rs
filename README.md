# Reticulum-rs

Rust implementation of the Reticulum network stack with reliability-first architecture.

## Project Goals
- Protocol correctness and deterministic behavior.
- Clear separation of transport core and daemon runtime.
- Portable core with proprietary drivers kept out-of-tree.

## Workspace Layout

```text
Reticulum-rs/
├── crates/
│   ├── reticulum/           # Protocol and transport core
│   └── reticulum-daemon/    # Runtime shell and bridge logic
├── docs/
│   ├── architecture/
│   ├── adr/
│   └── compatibility-contract.md
└── .github/workflows/ci.yml
```

## Transport Module Topology
- `transport::core`
- `transport::path`
- `transport::announce`
- `transport::jobs`
- `transport::wire`

## Build

```bash
cargo check --workspace --all-targets --all-features
cargo test --workspace --all-features
```

For a full target sweep (examples + benches + doc tests), use:

```bash
cargo test --workspace --all-targets --all-features
```

## Extension Boundary
External hardware integrations should implement traits in:
- `crates/reticulum/src/iface/driver.rs`

This keeps proprietary drivers outside the repository while preserving integration points.

## Compatibility
- Cross-repo contract: `docs/compatibility-contract.md`

## Governance
- Contribution guide: `CONTRIBUTING.md`
- Security policy: `SECURITY.md`
- Code owners: `.github/CODEOWNERS`

## License
MIT
