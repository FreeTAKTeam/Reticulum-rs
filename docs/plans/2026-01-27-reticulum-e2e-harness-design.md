# Reticulum Two-Daemon E2E Harness Design

**Goal:** Provide a repeatable CLI harness that spins up two real `reticulumd` daemons, sends a message between them, verifies delivery, and (optionally) validates announces.

## Architecture

We will add an `rnx e2e` subcommand that orchestrates two `reticulumd` processes on distinct RPC ports. The harness will:
- spawn daemons with isolated SQLite databases
- wait for “listening” output to confirm readiness
- send RPC calls over HTTP to daemon A (`send_message`, `announce_now`)
- poll daemon B (`list_messages`, `list_peers`) for delivery/announce visibility
- exit non-zero on failure and clean up child processes and temp DBs unless `--keep` is set

The harness will live in `src/bin/rnx.rs` and a small module `src/bin/rnx/e2e.rs` to keep process orchestration and RPC helpers contained. RPC calls use the existing HTTP protocol (`reticulum::rpc::http`) to keep compatibility with production `reticulumd`.

## Data Flow

1) Spawn daemon A and B with distinct ports and DB paths.  
2) Wait for readiness by scanning stdout for “listening on http://”.  
3) Send `send_message` to daemon A with destination `daemon-b`.  
4) Poll daemon B’s `list_messages` until the message appears or timeout.  
5) Optionally call `announce_now` on daemon A and poll daemon B’s `list_peers`.  
6) Tear down processes; emit a concise summary and exit code.

## Error Handling

- Timeout: includes ports and last response bodies.
- RPC failure: surfaces `error` payload from JSON response.
- Missing binary: instructs to `cargo build` or run via `cargo run --bin`.

## Testing

Keep tests lightweight:
- unit test for parsing daemon “ready” output (string matching)
- unit test for minimal RPC request/response parsing helper

Full integration can be added later if desired; the CLI harness itself is designed to be run manually or in CI.
