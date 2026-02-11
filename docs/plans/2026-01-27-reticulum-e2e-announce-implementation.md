# Reticulum E2E Announce Harness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Extend `rnx e2e` to exercise announce/peer flows via simulated delivery and verify peers appear in `list_peers`.

**Architecture:** Add helper builders for announce params and peer checks, then extend the harness flow to trigger announce on daemon A, simulate delivery to daemon B, and poll `list_peers` for the peer. Keep simulated delivery explicit in CLI output.

**Tech Stack:** Rust, clap, serde_json, existing RPC HTTP/msgpack helpers.

---

### Task 1: Add announce param builder + peer check helper tests

**Files:**
- Modify: `tests/e2e_harness_helpers.rs`
- Modify: `src/e2e_harness.rs`

**Step 1: Write the failing tests**

Append to `tests/e2e_harness_helpers.rs`:
```rust
use reticulum::e2e_harness::{build_announce_params, peer_present};

#[test]
fn announce_params_include_peer() {
    let params = build_announce_params("peer-123", Some(42));
    assert_eq!(params["peer"], "peer-123");
    assert_eq!(params["timestamp"], 42);
}

#[test]
fn peer_present_detects_peer() {
    let response = reticulum::rpc::RpcResponse {
        id: 1,
        result: Some(serde_json::json!({
            "peers": [
                {"peer": "peer-a"},
                {"peer": "peer-b"}
            ]
        })),
        error: None,
    };
    assert!(peer_present(&response, "peer-b"));
    assert!(!peer_present(&response, "peer-missing"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum --test e2e_harness_helpers announce_params_include_peer -v`
Expected: FAIL (missing helpers).

**Step 3: Write minimal implementation**

In `src/e2e_harness.rs`, add:
```rust
pub fn build_announce_params(peer: &str, timestamp: Option<i64>) -> serde_json::Value {
    serde_json::json!({
        "peer": peer,
        "timestamp": timestamp,
    })
}

pub fn peer_present(response: &crate::rpc::RpcResponse, peer: &str) -> bool {
    let Some(result) = response.result.as_ref() else { return false; };
    let Some(peers) = result.get("peers").and_then(|value| value.as_array()) else { return false; };
    peers.iter().any(|entry| {
        entry.get("peer").and_then(|value| value.as_str()) == Some(peer)
    })
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum --test e2e_harness_helpers announce_params_include_peer -v`
Expected: PASS

**Step 5: Commit**

```bash
git add tests/e2e_harness_helpers.rs src/e2e_harness.rs
git commit -m "feat: add announce helpers for e2e harness"
```

---

### Task 2: Extend rnx e2e flow with announce simulation

**Files:**
- Modify: `src/bin/rnx.rs`

**Step 1: Write the failing test**

Add a helper test in `tests/e2e_harness_helpers.rs`:
```rust
use reticulum::e2e_harness::simulated_announce_notice;

#[test]
fn simulated_announce_notice_mentions_simulated() {
    let note = simulated_announce_notice("127.0.0.1:4243", "127.0.0.1:4244");
    assert!(note.to_lowercase().contains("simulated"));
    assert!(note.contains("4243"));
    assert!(note.contains("4244"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum --test e2e_harness_helpers simulated_announce_notice_mentions_simulated -v`
Expected: FAIL (missing helper).

**Step 3: Write minimal implementation**

In `src/e2e_harness.rs`:
```rust
pub fn simulated_announce_notice(a_rpc: &str, b_rpc: &str) -> String {
    format!(
        "Simulated announce delivery: {} -> {}",
        a_rpc, b_rpc
    )
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum --test e2e_harness_helpers simulated_announce_notice_mentions_simulated -v`
Expected: PASS

**Step 5: Implement harness changes (no new tests required)**

In `src/bin/rnx.rs`:
- After message delivery, call `announce_now` on daemon A (or skip if not implemented).
- Simulate delivery to daemon B via `announce_received` with `build_announce_params`.
- Poll `list_peers` on daemon B using `peer_present`.
- Print `simulated_announce_notice` before the announce flow.

**Step 6: Commit**

```bash
git add src/bin/rnx.rs src/e2e_harness.rs tests/e2e_harness_helpers.rs
git commit -m "feat: add simulated announce flow to rnx e2e"
```

---

### Task 3: Document announce flow in usage

**Files:**
- Modify: `docs/plans/2026-01-27-reticulum-e2e-harness-design.md`

**Step 1: Append note**

Add a short note under Usage:
```
Simulated announce delivery is enabled during rnx e2e and verified via list_peers.
```

**Step 2: Commit**

```bash
git add docs/plans/2026-01-27-reticulum-e2e-harness-design.md
git commit -m "docs: note announce simulation in e2e"
```

---

### Task 4: Final verification

**Step 1: Run tests**

Run: `cargo test -p reticulum --test e2e_harness_helpers -v`
Expected: PASS
