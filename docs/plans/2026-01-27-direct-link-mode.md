# Direct Link Mode Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Enable full DIRECT link-mode interoperability with Python Reticulum nodes (handshake, link payload delivery, and UI-visible sent/delivered status).

**Architecture:** Reuse the existing link system in `reticulum` (link requests, proofs, session keys) and wire `reticulumd` to establish links before sending LXMF payloads. Inbound link payloads are converted into the existing `ReceivedData` flow so the daemon can decode and store messages consistently.

**Tech Stack:** Rust, `reticulum` transport/link modules, `reticulum-daemon`, SQLite, tokio.

---

### Task 1: Verify link payload delivery path with a failing test

**Files:**
- Create: `crates/reticulum/tests/link_payload_forwarding.rs`
- Modify: `crates/reticulum/src/transport.rs`

**Step 1: Write the failing test**

```rust
use reticulum::destination::{DestinationDesc, DestinationName};
use reticulum::identity::PrivateIdentity;
use reticulum::transport::{Transport, TransportConfig};
use reticulum::destination::link::LinkEvent;
use rand_core::OsRng;

#[tokio::test]
async fn link_payload_is_forwarded_to_received_data() {
    let identity = PrivateIdentity::new_from_rand(OsRng);
    let config = TransportConfig::new("test", &identity, true);
    let transport = Transport::new(config);

    // Create a destination desc to open a link
    let dest_identity = PrivateIdentity::new_from_rand(OsRng).as_identity().clone();
    let destination = DestinationDesc {
        identity: dest_identity,
        address_hash: *dest_identity.address_hash(),
        name: DestinationName::new("lxmf", "delivery"),
    };

    let link = transport.link(destination).await;

    // Simulate inbound link data by emitting a LinkEvent::Data on the link
    // (this will require a transport hook to forward link events to received_data)
    let mut rx = transport.received_data_events();

    // TODO: emit a LinkEvent::Data via the link object once the hook exists
    // Expected: received_data event with destination == destination.address_hash

    let _ = rx.recv().await.expect("received data");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum --test link_payload_forwarding -- --nocapture`

Expected: FAIL because link events are not forwarded to `received_data_tx`.

**Step 3: Write minimal implementation**

In `crates/reticulum/src/transport.rs`:
- Add a small async task inside `Transport::new` (or a new helper called from it) that subscribes to `in_link_events()` and forwards `LinkEvent::Data` to `received_data_tx`.
- When forwarding, use `event.address_hash` as `ReceivedData.destination` and the `LinkPayload` bytes as `ReceivedData.data`.

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum --test link_payload_forwarding -- --nocapture`

Expected: PASS.

**Step 5: Commit**

```bash
git add crates/reticulum/src/transport.rs crates/reticulum/tests/link_payload_forwarding.rs
git commit -m "feat(reticulum): forward link payloads to received data"
```

---

### Task 2: Add link-aware delivery in reticulumd (DIRECT path)

**Files:**
- Modify: `crates/reticulum-daemon/src/bin/reticulumd.rs`
- Modify: `crates/reticulum-daemon/src/lib.rs`
- Modify: `crates/reticulum-daemon/src/identity_store.rs` (if needed)

**Step 1: Write the failing test**

Create `crates/reticulum-daemon/tests/direct_link_delivery.rs`:

```rust
use reticulum_daemon::lxmf_bridge::{build_wire_message, decode_wire_message};
use reticulum::destination::{DestinationDesc, DestinationName};
use reticulum::identity::PrivateIdentity;
use reticulum::transport::{Transport, TransportConfig};
use rand_core::OsRng;

#[tokio::test]
async fn direct_send_uses_link_payloads() {
    let a = PrivateIdentity::new_from_rand(OsRng);
    let b = PrivateIdentity::new_from_rand(OsRng);

    let transport = Transport::new(TransportConfig::new("test", &a, true));

    let dest = DestinationDesc {
        identity: b.as_identity().clone(),
        address_hash: *b.address_hash(),
        name: DestinationName::new("lxmf", "delivery"),
    };

    let link = transport.link(dest).await;
    let payload = b"hello".to_vec();

    let packet = link.lock().await.data_packet(&payload).expect("packet");
    assert_eq!(packet.header.destination_type as u8, 0b11); // Link
}
```

Run: `cargo test -p reticulum-daemon --test direct_link_delivery -- --nocapture`

Expected: FAIL until daemon wiring uses link paths.

**Step 2: Implement link-aware delivery**

In `reticulumd.rs`:
- Expand `PeerCrypto` to store full `Identity` and `DestinationName` so we can construct `DestinationDesc` for the link.
- When `deliver()` is called:
  - Build `DestinationDesc` from peer identity + name (`lxmf.delivery`).
  - Call `transport.link(dest).await` in a spawned task.
  - Wait for `LinkStatus::Active` (subscribe to `transport.out_link_events()` and watch for `LinkEvent::Activated` for the link ID with timeout).
  - Build the LXMF wire payload (same as today) and send with `link.data_packet()` and `transport.send_packet()`.
  - For now, set “sent” when packet is queued; “delivered” when LXMF receipt arrives.

**Step 3: Run tests**

Run: `cargo test -p reticulum-daemon --test direct_link_delivery -- --nocapture`

Expected: PASS.

**Step 4: Commit**

```bash
git add crates/reticulum-daemon/src/bin/reticulumd.rs crates/reticulum-daemon/tests/direct_link_delivery.rs
git commit -m "feat(reticulumd): send direct messages via link"
```

---

### Task 3: Inbound DIRECT messages flow into daemon

**Files:**
- Modify: `crates/reticulum-daemon/src/bin/reticulumd.rs`

**Step 1: Write failing test**

Add `crates/reticulum-daemon/tests/direct_link_inbound.rs`:

```rust
use reticulum::destination::{DestinationDesc, DestinationName};
use reticulum::identity::PrivateIdentity;
use reticulum::transport::{Transport, TransportConfig};
use rand_core::OsRng;

#[tokio::test]
async fn inbound_link_payload_is_decoded_by_daemon() {
    let a = PrivateIdentity::new_from_rand(OsRng);
    let b = PrivateIdentity::new_from_rand(OsRng);

    let transport = Transport::new(TransportConfig::new("test", &a, true));

    let dest = DestinationDesc {
        identity: b.as_identity().clone(),
        address_hash: *b.address_hash(),
        name: DestinationName::new("lxmf", "delivery"),
    };

    let link = transport.link(dest).await;
    let payload = b"hello".to_vec();

    let packet = link.lock().await.data_packet(&payload).expect("packet");
    transport.send_packet(packet).await;

    // Expect: daemon receives and stores inbound message after link forwarding is wired.
}
```

Expected: FAIL until daemon consumes link data events.

**Step 2: Implement inbound link handling**

In `reticulumd.rs` main:
- Spawn a task subscribing to `transport.in_link_events()`.
- On `LinkEvent::Data(payload)`:
  - Prepend destination hash to payload (same as opportunistic) and pass to LXMF decode.
  - Store inbound message via `daemon.accept_inbound(record)`.

**Step 3: Run tests**

Run: `cargo test -p reticulum-daemon --test direct_link_inbound -- --nocapture`

Expected: PASS.

**Step 4: Commit**

```bash
git add crates/reticulum-daemon/src/bin/reticulumd.rs crates/reticulum-daemon/tests/direct_link_inbound.rs
git commit -m "feat(reticulumd): consume inbound link payloads"
```

---

### Task 4: Python interop smoke test (manual)

**Files:**
- Modify (optional): `tmp/python-lxmd/interop_sender.py`
- Modify (optional): `tmp/python-lxmd/interop_receiver.py`

**Step 1: Run reticulumd**

Set these local checkout paths first:

```bash
RETICULUM_RS_REPO=<local clone of https://github.com/FreeTAKTeam/Reticulum-rs>
LXMF_RS_REPO=<local clone of https://github.com/FreeTAKTeam/LXMF-rs>
```

```bash
"$RETICULUM_RS_REPO"/target/debug/reticulumd \
  --rpc 127.0.0.1:4243 \
  --transport 0.0.0.0:4242 \
  --db "$LXMF_RS_REPO"/tmp/host-reticulum.db \
  --announce-interval-secs 0
```

**Step 2: Send DIRECT from python**

```bash
"$LXMF_RS_REPO"/tmp/python-lxmd/.venv/bin/python -u \
  "$LXMF_RS_REPO"/tmp/python-lxmd/interop_sender.py \
  <CURRENT_DELIVERY_HASH> "hello direct"
```

Expected: reticulumd logs `rx data` and message appears in `list_messages` with `direction=in`.

**Step 3: Send DIRECT from daemon to python**

Use Weft UI or RPC `send_message` to python’s announced identity.

Expected: python receiver prints message; daemon `send` returns OK.

---

### Task 5: Clean up debug logging

**Files:**
- Modify: `crates/reticulum/src/transport.rs`
- Modify: `crates/reticulum-daemon/src/bin/reticulumd.rs`

Remove temporary `eprintln!` diagnostics added for path/link tracing once tests pass.

**Commit:**
```bash
git add crates/reticulum/src/transport.rs crates/reticulum-daemon/src/bin/reticulumd.rs
git commit -m "chore: remove link debug tracing"
```

---

## Notes / Constraints
- DIRECT delivery must use links for Python interop.
- Link payloads do not currently emit proofs; treat `sent` as “queued after link active” and rely on LXMF receipts for `delivered`.
- This plan assumes link handling already exists in `reticulum` and just needs wiring in daemon + forwarding of link data events.
