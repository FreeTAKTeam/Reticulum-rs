# Daemon LXMF Transport Interop Design

**Goal:** Make `reticulumd` the authoritative LXMF router and Reticulum transport host so Weft and Python peers interoperate with full feature parity.

**Architecture:** The daemon owns transport, LXMF routing, and persistence, and exposes RPC for Weft. Python peers connect via the transport interface, using standard LXMF packets. RPC surfaces remain stable but now back a real router rather than simulated deliveries.

**Tech Stack:** Rust (`reticulum-rs`), SQLite, RPC over HTTP with msgpack frames, Reticulum TCP interface.

## Architecture Overview

- **Transport host:** `reticulumd` spawns a TCP server interface on `--transport` (e.g. `0.0.0.0:4242`) and handles Reticulum packet IO.
- **LXMF router:** Runs inside daemon; decodes inbound packets, stores messages, emits events, and drives outbound deliveries.
- **Persistence:** Messages, peers, receipts, and status transitions stored in SQLite via existing store modules (extend if needed).
- **RPC layer:** `send_message`, `list_messages`, `list_peers`, `announce_now`, etc., are backed by real LXMF transport; events (`inbound`, `outbound`, `receipt`, `announce_received`) are emitted for UI updates.

## Data Flow

**Inbound (Python → Weft):**
- Python sends LXMF via Reticulum TCP transport.
- Daemon transport receives packet → router validates/decodes → store message → emit `inbound` event.
- Receipt handling updates status and emits `receipt` event.

**Outbound (Weft → Python):**
- Weft `send_message` RPC → daemon builds LXMF message with content/title/fields.
- Router queues and sends over transport → store status `sending` → emit `outbound`.
- On receipt, status becomes `delivered` and `receipt` event emits.

**Announces/Peers:**
- Transport announces processed by router → peers stored → `announce_received` event.
- `announce_now` triggers immediate announce and updates last‑announce.

## Error Handling

- **Transport down:** messages stored as `queued`, retries scheduled; UI shows pending.
- **Invalid arguments:** RPC returns `INVALID_ARGUMENT` and no store write.
- **Duplicate inbound:** idempotent store behavior prevents duplication, updates last_seen.
- **Receipt timeout:** status transitions to `timeout` and event emits.

## Testing Strategy

- **Unit:**
  - RPC arg builder includes `--transport`.
  - LXMF encode/decode through daemon transport.
  - Status transitions on send/receipt.
  - Peer announces update store + emit events.
- **Integration:**
  - Start daemon with `--transport`; inject LXMF packet and assert `list_messages`.
  - `send_message` produces transport egress.
- **E2E Manual:**
  - Weft → Python receiver shows inbound.
  - Python sender → Weft UI shows inbound + receipts.

## Migration Steps

1) Add transport host to `reticulumd` (`--transport`).
2) Embed LXMF router in daemon and connect to transport interface.
3) Map RPC `send_message`/`list_messages` to router/store.
4) Implement inbound event emission and receipt handling.
5) Announce + peers wiring and event emission.
6) Update Weft client only if event payloads change.
7) Run tests + Python E2E.
