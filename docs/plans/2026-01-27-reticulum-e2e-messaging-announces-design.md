# E2E Messaging + Announce Flows (TCP-first, no local Python)

## Goal
Deliver end-to-end messaging and announce flows that interoperate with Python Reticulum peers without requiring Python locally. Start with TCP transport, then extend to other transports later.

## Scope
- TCP-only peering for interoperability validation.
- Daemon RPC for send/receive message and announce operations.
- Event stream for inbound/outbound message and announce events.
- Periodic announces + manual “announce now” endpoint.
- Minimal UI wiring in Weft to trigger announces and render event feedback.

## Non-Goals (for this phase)
- UDP/LoRa/audio transports.
- Full Sideband/Columba feature parity.
- Rich identity UX or persistence beyond current storage.

## Architecture
- **reticulumd** runs locally and handles all networking via Rust transport.
- **Weft** uses RPC over HTTP to drive and observe messaging/announce flows.
- **/events** endpoint provides a simple poll-based event stream.

Data flow:
1) Weft → RPC → reticulumd → transport → external Python peer.
2) External peer → transport → reticulumd → /events → Weft.

## RPC Methods
- `send_message` (existing): send LXMF payload, emit `message_sent`.
- `receive_message` (dev-only): inject inbound (local testing), emit `inbound`.
- `announce_now` (new): force announce, emit `announce_sent`.
- `add_peer` / `set_peers` (new): set TCP peers at runtime.
- `status` (extended): include `peers_connected`, `last_announce_ts`.

## Event Model
Events are MessagePack-framed. Add a small in-memory queue for `/events` to avoid loss between polls.

- `inbound`: `{ message: MessageRecord }`
- `message_sent`: `{ message_id, destination, timestamp }`
- `announce_sent`: `{ destination_hash, timestamp }`
- `announce_received`: `{ peer, timestamp }`
- `peer_connected`: `{ peer }`
- `peer_disconnected`: `{ peer, reason? }`

## Announce Scheduling
- Add a scheduler loop in `reticulumd` honoring `announce_interval_secs`.
- On tick, emit `announce_sent`.
- Manual `announce_now` RPC bypasses timer.

## Weft UX
- Dev panel: “Announce Now” button + recent announce events.
- Background event polling via `/events` updates message list and status.

## Testing Strategy
### Automated (Rust)
- In-process TCP transport test: A ↔ B, send message, verify inbound + event.
- Announce test: A announces, B receives, verify `announce_received` event.
- Scheduler test: verify periodic announce fires and emits event.

### Manual E2E
- Run `reticulumd` and connect to an external Python node over TCP.
- Send a message and trigger announce from Weft; verify remote receipt.
- Verify inbound messages appear in Weft via `/events` without manual refresh.

## Success Criteria
- Weft can send/receive messages via `reticulumd` to a Python peer over TCP.
- Announce flows work (manual + periodic) with visible events.
- Event stream is reliable for user-visible updates.
