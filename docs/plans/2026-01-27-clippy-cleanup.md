# Clippy Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Eliminate all `cargo clippy` warnings in Reticulum-rs across src, tests, and examples without changing behavior unless explicitly noted.

**Architecture:** Apply mechanical refactors first (formatting, redundant names, needless clones/borrows/returns). Then address low-risk API improvements (Default + is_empty) with targeted tests. Finally handle behavior- or layout-affecting suggestions (boxing large enum variants, Arc non-Send/Sync) in isolated tasks with explicit confirmation.

**Tech Stack:** Rust, Clippy, Tokio, rusqlite.

---

### Task 1: Mechanical clippy fixes (safe refactors)

**Files:**
- Modify: `src/iface/kaonic/kaonic_grpc.rs`
- Modify: `src/transport/link_table.rs`
- Modify: `src/crypt/fernet.rs`
- Modify: `src/destination/link.rs`
- Modify: `src/destination.rs`
- Modify: `src/hash.rs`
- Modify: `src/identity.rs`
- Modify: `src/iface/hdlc.rs`
- Modify: `src/utils/cache_set.rs`
- Modify: `src/utils/resolver.rs`
- Modify: `src/serde.rs`
- Modify: `src/transport/path_requests.rs`
- Modify: `src/transport.rs`
- Modify: `examples/multihop.rs`
- Modify: `tests/tcp_hdlc_test.rs`

**Step 1: Run clippy to confirm warnings**

Run: `cargo clippy -p reticulum --all-targets --all-features`
Expected: warnings shown for redundant_field_names, precedence, needless_return, clone_on_copy, redundant_closure, needless_borrow, etc.

**Step 2: Apply mechanical edits (no behavior change)**

Examples of exact changes to make (apply all applicable):

- Replace redundant field names:
  ```rust
  module: module,
  ```
  with
  ```rust
  module,
  ```

- Fix operator precedence:
  ```rust
  word >> i * 8
  ```
  to
  ```rust
  word >> (i * 8)
  ```

- Replace `return` with expression:
  ```rust
  return Ok(PlainText { 0: msg });
  ```
  to
  ```rust
  Ok(PlainText(msg))
  ```

- Replace tuple struct initializers:
  ```rust
  Self { 0: item }
  ```
  to
  ```rust
  Self(item)
  ```

- Replace needless clones on Copy types:
  ```rust
  address_hash.clone()
  ```
  to
  ```rust
  address_hash
  ```

- Replace needless borrows:
  ```rust
  buffer.read(&mut address.as_mut_slice())
  ```
  to
  ```rust
  buffer.read(address.as_mut_slice())
  ```

- Collapse `if` or `match` single-branch blocks where suggested by clippy.

**Step 3: Re-run clippy**

Run: `cargo clippy -p reticulum --all-targets --all-features`
Expected: no warnings from the refactors in Step 2; remaining warnings are limited to API shape suggestions (Default/is_empty, large enum variant, Arc non-Send/Sync).

**Step 4: Commit**

```bash
git add src/iface/kaonic/kaonic_grpc.rs src/transport/link_table.rs src/crypt/fernet.rs src/destination/link.rs src/destination.rs src/hash.rs src/identity.rs src/iface/hdlc.rs src/utils/cache_set.rs src/utils/resolver.rs src/serde.rs src/transport/path_requests.rs src/transport.rs examples/multihop.rs tests/tcp_hdlc_test.rs
git commit -m "chore: apply mechanical clippy fixes"
```

---

### Task 2: Add Default + is_empty helpers (API additive, low risk)

**Files:**
- Modify: `src/buffer.rs`
- Modify: `src/crypt/fernet.rs`
- Modify: `src/destination/link.rs`
- Modify: `src/destination/link_map.rs`
- Modify: `src/hash.rs`
- Modify: `src/transport/path_table.rs`
- Modify: `src/transport/discovery.rs`
- Modify: `src/utils/resolver.rs`
- Test: `tests/api_helpers.rs` (new)

**Step 1: Write failing test**

Create `tests/api_helpers.rs`:
```rust
use reticulum::buffer::StaticBuffer;
use reticulum::crypt::fernet::Token;
use reticulum::destination::link::LinkPayload;
use reticulum::destination::link_map::LinkMap;
use reticulum::hash::AddressHash;
use reticulum::transport::path_table::PathTable;
use reticulum::transport::discovery::DiscoveryCache;
use reticulum::utils::resolver::Resolver;

#[test]
fn helper_methods_exist_and_work() {
    let buffer = StaticBuffer::<64>::default();
    assert!(buffer.is_empty());

    let token = Token::from(b"".as_ref());
    assert!(token.is_empty());

    let payload = LinkPayload::default();
    assert!(payload.is_empty());

    let map = LinkMap::default();
    assert!(map.is_empty());

    let hash = AddressHash::default();
    assert!(hash.is_empty());

    let table = PathTable::default();
    assert!(table.is_empty());

    let discovery = DiscoveryCache::default();
    assert!(discovery.is_empty());

    let resolver = Resolver::default();
    assert!(resolver.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum --test api_helpers -v`
Expected: FAIL (missing Default/is_empty)

**Step 3: Implement helpers**

Add `Default` impls using `Self::new()` where available, and add `is_empty()` (returning `len() == 0`) to the listed types.

**Step 4: Re-run test**

Run: `cargo test -p reticulum --test api_helpers -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/buffer.rs src/crypt/fernet.rs src/destination/link.rs src/destination/link_map.rs src/hash.rs src/transport/path_table.rs src/transport/discovery.rs src/utils/resolver.rs tests/api_helpers.rs
git commit -m "feat: add default and is_empty helpers"
```

---

### Task 3: Address Arc non-Send/Sync in reticulumd and announce_scheduler test

**Files:**
- Modify: `src/bin/reticulumd.rs`
- Modify: `tests/announce_scheduler.rs`

**Step 1: Write failing test (lint-as-test)**

Run: `cargo clippy -p reticulum --all-targets --all-features`
Expected: warning `arc_with_non_send_sync` in `reticulumd.rs` and `tests/announce_scheduler.rs`

**Step 2: Implement minimal fix**

- Replace `Arc<RpcDaemon>` with `Rc<RpcDaemon>` where the value does not cross threads. Example:
  ```rust
  use std::rc::Rc;
  let daemon = Rc::new(RpcDaemon::with_store(store, "local".into()));
  ```

**Step 3: Re-run clippy**

Run: `cargo clippy -p reticulum --all-targets --all-features`
Expected: warning removed

**Step 4: Commit**

```bash
git add src/bin/reticulumd.rs tests/announce_scheduler.rs
git commit -m "chore: use rc for non-send daemon"
```

---

### Task 4: Evaluate large enum variant suggestion (LinkEvent)

**Files:**
- Modify (optional): `src/destination/link.rs`
- Test (if change): `tests/link_event_layout.rs`

**Step 1: Decide with confirmation**

- If keeping as-is, add `#[allow(clippy::large_enum_variant)]` on `LinkEvent` with a short comment.
- If changing, box the large variant:
  ```rust
  Data(Box<LinkPayload>)
  ```
  and update call sites.

**Step 2: If changing, add a test**

Create `tests/link_event_layout.rs` (only if boxed):
```rust
use reticulum::destination::link::{LinkEvent, LinkPayload};

#[test]
fn link_event_handles_boxed_payload() {
    let payload = LinkPayload::default();
    let event = LinkEvent::Data(Box::new(payload));
    match event {
        LinkEvent::Data(_) => {}
        _ => panic!("expected data event"),
    }
}
```

**Step 3: Run test and clippy**

Run: `cargo test -p reticulum --test link_event_layout -v` (only if new test)
Run: `cargo clippy -p reticulum --all-targets --all-features`

**Step 4: Commit**

```bash
git add src/destination/link.rs tests/link_event_layout.rs
git commit -m "chore: address link event size warning"
```

---

### Task 5: Final verification

**Step 1: Run full test suite**

Run: `cargo test -p reticulum`
Expected: PASS

**Step 2: Run clippy**

Run: `cargo clippy -p reticulum --all-targets --all-features`
Expected: No warnings

**Step 3: Report results**

Summarize changes + remaining warnings (if any).

