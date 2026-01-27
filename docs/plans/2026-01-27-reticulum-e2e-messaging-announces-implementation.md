# E2E Messaging + Announce Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Ship TCP-first end-to-end messaging and announce flows via reticulumd RPC + Weft UI with reliable event updates.

**Architecture:** reticulumd handles transport, messaging, and announce scheduling; Weft drives via RPC and listens via /events polling. A bounded event queue avoids missing events between polls. Announce flows are triggered by a scheduler and a manual RPC endpoint.

**Tech Stack:** Rust (reticulum-rs, tokio), MessagePack framing, Expo/React Native, TypeScript.

> **Note:** Skill recommends a worktree, but user requested no worktrees. Execute in-place.

---

### Task 1: Add a bounded event queue to RPC daemon

**Files:**
- Modify: `src/rpc/mod.rs`
- Test: `tests/rpc_events.rs`

**Step 1: Write the failing test**

Create `tests/rpc_events.rs`:
```rust
use reticulum::rpc::{RpcDaemon, RpcEvent};

#[test]
fn rpc_event_queue_drains_in_fifo_order() {
    let daemon = RpcDaemon::test_instance();
    daemon.push_event(RpcEvent { event_type: "one".into(), payload: serde_json::json!({"i": 1}) });
    daemon.push_event(RpcEvent { event_type: "two".into(), payload: serde_json::json!({"i": 2}) });

    let first = daemon.take_event().expect("first");
    let second = daemon.take_event().expect("second");

    assert_eq!(first.event_type, "one");
    assert_eq!(second.event_type, "two");
}

#[test]
fn rpc_event_queue_returns_none_when_empty() {
    let daemon = RpcDaemon::test_instance();
    assert!(daemon.take_event().is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum --test rpc_events rpc_event_queue_drains_in_fifo_order -v`
Expected: FAIL with missing `push_event` or similar.

**Step 3: Write minimal implementation**

Update `src/rpc/mod.rs` to add a bounded queue and helper:
```rust
use std::collections::VecDeque;

pub struct RpcDaemon {
    // ...
    event_queue: Mutex<VecDeque<RpcEvent>>,
}

impl RpcDaemon {
    pub fn with_store(store: MessagesStore, identity_hash: String) -> Self {
        let (events, _rx) = broadcast::channel(64);
        Self {
            // ...
            events,
            last_event: Mutex::new(None),
            event_queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn push_event(&self, event: RpcEvent) {
        let mut guard = self.event_queue.lock().expect("event_queue mutex poisoned");
        if guard.len() >= 32 {
            guard.pop_front();
        }
        guard.push_back(event);
    }

    pub fn take_event(&self) -> Option<RpcEvent> {
        let mut guard = self.event_queue.lock().expect("event_queue mutex poisoned");
        guard.pop_front()
    }
}
```

Update existing event emitters to call `push_event` in addition to broadcasts.

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum --test rpc_events rpc_event_queue_drains_in_fifo_order -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/rpc/mod.rs tests/rpc_events.rs
git commit -m "feat: add rpc event queue"
```

---

### Task 2: Update /events handler to drain queue

**Files:**
- Modify: `src/rpc/http.rs`
- Test: `tests/rpc_http.rs`

**Step 1: Write the failing test**

Extend `tests/rpc_http.rs`:
```rust
#[test]
fn rpc_http_events_drains_queue() {
    let store = MessagesStore::in_memory().unwrap();
    let daemon = RpcDaemon::with_store(store, "daemon".into());
    daemon.push_event(RpcEvent { event_type: "one".into(), payload: serde_json::json!({"i": 1}) });

    let request_bytes = b"GET /events HTTP/1.1\r\nHost: localhost\r\n\r\n".to_vec();
    let response = reticulum::rpc::http::handle_http_request(&daemon, &request_bytes).unwrap();
    let body_start = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    let event: RpcEvent = decode_frame(&response[body_start..]).unwrap();

    assert_eq!(event.event_type, "one");
    assert!(daemon.take_event().is_none());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum --test rpc_http rpc_http_events_drains_queue -v`
Expected: FAIL (queue not drained or wrong event source)

**Step 3: Write minimal implementation**

Update `src/rpc/http.rs` to use `take_event()` from the queue when handling `/events`.

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum --test rpc_http rpc_http_events_drains_queue -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/rpc/http.rs tests/rpc_http.rs
git commit -m "feat: drain rpc event queue via /events"
```

---

### Task 3: Add announce_now RPC + event emission

**Files:**
- Modify: `src/rpc/mod.rs`
- Modify: `src/transport.rs` (if required)
- Test: `tests/rpc_announces.rs`

**Step 1: Write the failing test**

Create `tests/rpc_announces.rs`:
```rust
use reticulum::rpc::RpcDaemon;

#[test]
fn announce_now_emits_event() {
    let daemon = RpcDaemon::test_instance();
    let resp = daemon.handle_rpc(reticulum::rpc::RpcRequest {
        id: 1,
        method: "announce_now".into(),
        params: None,
    }).unwrap();

    assert!(resp.result.is_some());
    let event = daemon.take_event().expect("announce event");
    assert_eq!(event.event_type, "announce_sent");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum --test rpc_announces announce_now_emits_event -v`
Expected: FAIL (method not implemented)

**Step 3: Write minimal implementation**

In `src/rpc/mod.rs`, add `announce_now` to `handle_rpc`:
```rust
"announce_now" => {
    // TODO: call transport once wired; for now emit event
    let event = RpcEvent { event_type: "announce_sent".into(), payload: json!({"timestamp": timestamp}) };
    self.push_event(event.clone());
    let _ = self.events.send(event);
    Ok(RpcResponse { id: request.id, result: Some(json!({"announce_id": request.id})), error: None })
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum --test rpc_announces announce_now_emits_event -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/rpc/mod.rs tests/rpc_announces.rs
git commit -m "feat: add announce_now rpc"
```

---

### Task 4: Wire announce scheduler in reticulumd

**Files:**
- Modify: `src/bin/reticulumd.rs`
- Test: `tests/reticulumd_announce_scheduler.rs`

**Step 1: Write the failing test**

Create `tests/reticulumd_announce_scheduler.rs`:
```rust
use reticulum::rpc::RpcDaemon;

#[test]
fn announce_scheduler_emits_events() {
    let daemon = RpcDaemon::test_instance();
    daemon.schedule_announce_for_test(1); // helper to trigger
    let event = daemon.take_event().expect("announce event");
    assert_eq!(event.event_type, "announce_sent");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum --test reticulumd_announce_scheduler announce_scheduler_emits_events -v`
Expected: FAIL (missing schedule_announce_for_test)

**Step 3: Write minimal implementation**

Add a test-only helper on `RpcDaemon` to trigger announce scheduling; then in `reticulumd` spawn a tokio task using `announce_interval_secs` to call `announce_now` handler.

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum --test reticulumd_announce_scheduler announce_scheduler_emits_events -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/bin/reticulumd.rs tests/reticulumd_announce_scheduler.rs src/rpc/mod.rs
git commit -m "feat: add announce scheduler"
```

---

### Task 5: Add Weft daemon announce RPC + dev panel button

**Files:**
- Modify: `app/lib/daemon-weft.ts`
- Modify: `app/app/(tabs)/settings/dev-panel.tsx`
- Test: `app/__tests__/daemon-weft.test.ts`

**Step 1: Write the failing test**

Extend `app/__tests__/daemon-weft.test.ts`:
```ts
test("announceNow calls daemon announce_now", async () => {
  mockCall.mockResolvedValue({ id: 1, result: { announce_id: 1 }, error: null });
  const api = createDaemonApi({ host: "127.0.0.1", port: 4243 });
  await api.announceNow();
  expect(mockCall).toHaveBeenCalledWith("announce_now", null);
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run --environment node app/__tests__/daemon-weft.test.ts`
Expected: FAIL (announceNow missing)

**Step 3: Write minimal implementation**

Update `app/lib/daemon-weft.ts` to add:
```ts
async function announceNow(): Promise<EngineResult<{ announce_id: number }>> {
  const resp = await client.call("announce_now", null);
  if (resp.error) return { ok: false, result: null, error: resp.error };
  return { ok: true, result: { announce_id: (resp.result as any)?.announce_id ?? Date.now() }, error: null };
}
```

Update dev panel to add an “Announce Now” button and show a success toast.

**Step 4: Run test to verify it passes**

Run: `npx vitest run --environment node app/__tests__/daemon-weft.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add app/lib/daemon-weft.ts app/app/(tabs)/settings/dev-panel.tsx app/__tests__/daemon-weft.test.ts
git commit -m "feat: add announce now to daemon api"
```

---

### Task 6: Update WeftContext to handle announce events

**Files:**
- Modify: `app/contexts/WeftContext.tsx`
- Test: `app/__tests__/weft-events.test.ts`

**Step 1: Write the failing test**

Create `app/__tests__/weft-events.test.ts`:
```ts
import { expect, test } from "vitest";
import { handleDaemonEventForTest } from "../contexts/WeftContext";

test("announce_sent event updates debug log", () => {
  const state = handleDaemonEventForTest({ event_type: "announce_sent", payload: { timestamp: 1 } });
  expect(state.debugLog[0]?.message).toContain("announce_sent");
});
```

**Step 2: Run test to verify it fails**

Run: `npx vitest run --environment node app/__tests__/weft-events.test.ts`
Expected: FAIL (helper missing)

**Step 3: Write minimal implementation**

Add a test-only helper to process daemon events and update debug log (or add an announce feed in state).

**Step 4: Run test to verify it passes**

Run: `npx vitest run --environment node app/__tests__/weft-events.test.ts`
Expected: PASS

**Step 5: Commit**

```bash
git add app/contexts/WeftContext.tsx app/__tests__/weft-events.test.ts
git commit -m "feat: handle announce events in Weft"
```

---

### Task 7: Manual E2E checklist (no Python locally)

**Files:**
- Modify: `docs/plans/2026-01-27-reticulum-e2e-messaging-announces-implementation.md`

**Step 1: Add checklist section**

Append checklist steps for running `reticulumd`, configuring Weft daemon host/port, sending message, and triggering announce. Include expected logs and UI state changes.

**Step 2: Commit**

```bash
git add docs/plans/2026-01-27-reticulum-e2e-messaging-announces-implementation.md
git commit -m "docs: add e2e manual checklist"
```

---

## Manual E2E Checklist (no Python locally)

### reticulumd (daemon)
1. Build and run:
   - `cargo run -p reticulum --bin reticulumd -- --rpc 127.0.0.1:4243 --announce-interval-secs 10`
2. Confirm it is listening:
   - Expect: `reticulumd listening on http://127.0.0.1:4243`
3. Smoke /events:
   - `curl -i http://127.0.0.1:4243/events`
   - Expect: `204 No Content` when no events.

### Weft (app)
1. Configure daemon:
   - Enable daemon mode; set host `127.0.0.1`, port `4243`.
2. Dev panel checks:
   - Use “Announce Now” -> expect toast success + debug log entry containing `announce_sent`.
   - Use “Inject Inbound” -> expect new inbound message in list.

### End-to-end message flow
1. Send message from app (any contact/destination):
   - Expect send succeeds and message appears in thread.
2. Inject inbound (dev panel):
   - Expect message list updates + peer list refreshes.

### Announce scheduler
1. With `--announce-interval-secs 10`:
   - Wait ~10 seconds.
   - Expect periodic `announce_sent` debug log entries in Weft.

### Failure modes to watch
- `/events` returns 204 repeatedly while announces are triggered.
- Announce Now returns error from daemon.
- Messages list does not refresh after inbound injection.

## Two Daemon E2E

### Start Daemon A (app-facing)
```
cargo run -p reticulum --bin reticulumd -- --rpc 127.0.0.1:4243 --db /tmp/reticulum-a.db --announce-interval-secs 10
```

### Start Daemon B (peer)
```
cargo run -p reticulum --bin reticulumd -- --rpc 127.0.0.1:4244 --db /tmp/reticulum-b.db --announce-interval-secs 15
```

### Weft setup
- Enable daemon mode
- Host: `127.0.0.1`
- Port: `4243`

### Announce + inbound
- Use Dev Panel “Announce Now” → expect debug log entry `announce_sent`.
- Use Dev Panel “Inject Inbound” → expect message list update.

### Relay latest inbound (A → B)
From Weft repo:
```
FROM_PORT=4243 TO_PORT=4244 node app/scripts/relay-message.ts
```
Expect: “Relayed latest inbound…” message.

### Verify daemon B
```
printf 'POST /rpc HTTP/1.1\r\nHost: localhost\r\nContent-Length: %s\r\n\r\n' 0 | nc 127.0.0.1 4244
```
Then use Weft (pointed to B) or your RPC client to `list_messages` and confirm the relayed message exists.
