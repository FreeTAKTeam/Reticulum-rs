# Reticulum Two-Daemon E2E Harness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add an `rnx e2e` CLI that runs two real `reticulumd` daemons, sends a message, and verifies delivery (plus optional announce checks).

**Architecture:** Extend `rnx` to a multi-command CLI with an `e2e` subcommand. Implement a small helper module to spawn daemons, wait for readiness, send HTTP RPC calls, and poll results. Keep tests lightweight and focused on parsing helpers.

**Tech Stack:** Rust (clap, std::process, serde_json), existing `reticulum::rpc::http` helpers.

---

### Task 1: Add readiness parsing helper + tests

**Files:**
- Create: `tests/e2e_harness_helpers.rs`
- Modify: `src/bin/rnx.rs`

**Step 1: Write the failing test**

Create `tests/e2e_harness_helpers.rs`:
```rust
use reticulum::bin::e2e::is_ready_line;

#[test]
fn ready_line_detects_daemon_listening() {
    assert!(is_ready_line("reticulumd listening on http://127.0.0.1:4243"));
    assert!(!is_ready_line("starting daemon..."));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum --test e2e_harness_helpers ready_line_detects_daemon_listening -v`  
Expected: FAIL (missing helper/module).

**Step 3: Write minimal implementation**

In `src/bin/rnx.rs` (or a new module `src/bin/rnx/e2e.rs`), add:
```rust
pub fn is_ready_line(line: &str) -> bool {
    line.contains("listening on http://")
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum --test e2e_harness_helpers ready_line_detects_daemon_listening -v`  
Expected: PASS

**Step 5: Commit**

```bash
git add tests/e2e_harness_helpers.rs src/bin/rnx.rs
git commit -m "test: add rnx e2e readiness helper"
```

---

### Task 2: Add HTTP RPC helper + tests

**Files:**
- Modify: `tests/e2e_harness_helpers.rs`
- Modify: `src/bin/rnx.rs`

**Step 1: Write the failing test**

Append to `tests/e2e_harness_helpers.rs`:
```rust
use serde_json::json;
use reticulum::bin::e2e::{build_rpc_body, parse_rpc_response};

#[test]
fn rpc_helpers_roundtrip() {
    let body = build_rpc_body(1, "status", None);
    assert!(body.contains("\"method\":\"status\""));

    let resp = json!({"id":1,"result":{"ok":true},"error":null}).to_string();
    let parsed = parse_rpc_response(&resp).expect("parse");
    assert_eq!(parsed.id, 1);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum --test e2e_harness_helpers rpc_helpers_roundtrip -v`  
Expected: FAIL (missing helpers).

**Step 3: Write minimal implementation**

In `src/bin/rnx.rs` (or `src/bin/rnx/e2e.rs`), implement:
```rust
pub fn build_rpc_body(id: u64, method: &str, params: Option<serde_json::Value>) -> String { /* ... */ }
pub fn parse_rpc_response(input: &str) -> Result<RpcResponse, serde_json::Error> { /* ... */ }
```
Use the existing `RpcRequest`/`RpcResponse` structs.

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum --test e2e_harness_helpers rpc_helpers_roundtrip -v`  
Expected: PASS

**Step 5: Commit**

```bash
git add tests/e2e_harness_helpers.rs src/bin/rnx.rs
git commit -m "feat: add rnx e2e rpc helpers"
```

---

### Task 3: Implement `rnx e2e` harness (spawn + send + verify)

**Files:**
- Modify: `src/bin/rnx.rs`

**Step 1: Write the failing test**

Add a minimal CLI parse test to `tests/e2e_harness_helpers.rs`:
```rust
use clap::Parser;
use reticulum::bin::e2e::Cli;

#[test]
fn cli_parses_e2e() {
    let cli = Cli::parse_from(["rnx", "e2e"]);
    assert!(matches!(cli.command, reticulum::bin::e2e::Command::E2e { .. }));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum --test e2e_harness_helpers cli_parses_e2e -v`  
Expected: FAIL (no CLI).

**Step 3: Write minimal implementation**

In `src/bin/rnx.rs`:
- define `Cli` with `Command::E2e { a_port, b_port, timeout_secs, keep }`.
- spawn two `reticulumd` processes using `Command::new("reticulumd")` (or `cargo run --bin reticulumd` if you prefer local builds).
- wait for readiness by reading stdout lines and applying `is_ready_line`.
- send `send_message` RPC to daemon A via simple HTTP POST.
- poll daemon B `list_messages` until message appears or timeout.
- print success and exit 0, otherwise return error.

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum --test e2e_harness_helpers cli_parses_e2e -v`  
Expected: PASS

**Step 5: Commit**

```bash
git add src/bin/rnx.rs tests/e2e_harness_helpers.rs
git commit -m "feat: add rnx e2e harness"
```

---

### Task 4: Document usage

**Files:**
- Modify: `docs/plans/2026-01-27-reticulum-e2e-harness-design.md`

**Step 1: Add usage section**

Append:
```
## Usage

```
```
cargo run --bin rnx -- e2e --a-port 4243 --b-port 4244 --timeout-secs 5
```
```
```

**Step 2: Commit**

```bash
git add docs/plans/2026-01-27-reticulum-e2e-harness-design.md
git commit -m "docs: add rnx e2e usage"
```

---

### Task 5: Final verification

**Step 1: Run tests**

Run: `cargo test -p reticulum --test e2e_harness_helpers -v`  
Expected: PASS
