# Reticulum-rs Architecture

## Core/Runtime Split
- `crates/reticulum`: protocol, routing, packet and transport logic.
- `crates/reticulum-daemon`: daemon process concerns and service bridge.

## Reliability Rules
- Typed errors across boundaries.
- No panic-based control flow in runtime paths.
- Deterministic, test-backed transport and RPC behaviors.

## Driver Model
- In-tree drivers remain generic and open.
- Proprietary adapters are out-of-tree via `iface::driver` traits.
