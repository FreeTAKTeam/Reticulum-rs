# Reticulum-rs <-> LXMF-rs Compatibility Contract

## Version Mapping
- `reticulum-rs` `0.2.x` is intended to pair with `lxmf` `0.2.x`.

## Transport/RPC Invariants
- Packet and announce handling must remain deterministic.
- RPC framing remains stable (`rpc::codec` frame semantics).
- Invalid inputs must return typed errors and must not panic.

## Release Gate
Release requires:
1. Core + daemon tests pass.
2. Cross-repo LXMF compatibility job passes.
3. Compatibility docs updated with exact versions.
