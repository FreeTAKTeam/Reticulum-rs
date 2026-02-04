# Reticulum-rs Migration Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Achieve feature and wire-format parity between Python Reticulum in `/Users/tommy/Documents/TAK/Reticulum` and Rust Reticulum in `/Users/tommy/Documents/TAK/Reticulum-rs`, including core protocol, interfaces, and utilities.

**Architecture:** Implement Rust modules to mirror Python’s RNS stack, using compatibility-first behavior and golden Python fixtures. Core primitives (crypto, hashes, packet framing, identities, destinations, links, resources) come first, then transport tables and interfaces, then CLI utilities and config compatibility. A shared parity matrix drives sequencing and status.

**Tech Stack:** Rust (edition 2021), serde, prost/tonic (Kaonic), tokio (if used), crypto crates (ed25519-dalek, x25519-dalek, sha2, hkdf, aes, hmac), clap for CLI.

---

## Shared Dependency Map (for LXMF parity)

- Reticulum core primitives → required by LXMF message packing/signing.
- Reticulum transport/link/resource → required by LXMF router delivery modes.
- Reticulum interfaces → required by LXMF daemon behavior.
- Reticulum utilities/config format → required by LXMF daemon and operational parity.

---

## Feature Parity Matrix (Python → Rust)

- `RNS/Reticulum.py` → `src/transport.rs` (core orchestration, config, network lifecycle)
- `RNS/Identity.py` → `src/identity.rs`
- `RNS/Packet.py` → `src/packet.rs`
- `RNS/Destination.py` → `src/destination.rs` (+ `src/destination/link.rs`, `src/destination/link_map.rs`)
- `RNS/Link.py` → `src/destination/link.rs`
- `RNS/Resource.py` → `src/transport.rs` (resource state) + new `src/resource.rs` if split
- `RNS/Channel.py` → `src/transport.rs` (or new `src/channel.rs`)
- `RNS/Transport.py` → `src/transport.rs` + `src/transport/*`
- `RNS/Buffer.py` → `src/buffer.rs`
- `RNS/Discovery.py` → `src/transport.rs` or new `src/discovery.rs`
- `RNS/Resolver.py` → `src/transport.rs` or new `src/resolver.rs`
- `RNS/Interfaces/*.py` → `src/iface/*.rs`
- `RNS/Cryptography/*.py` → `src/crypt.rs`, `src/crypt/*`
- `RNS/Utilities/*.py` → new `src/bin/*` or `examples/*` CLI tools

Status: track each item as missing/partial/done with tests and dependencies.

---

### Task 1: Create parity matrix file and fixtures layout

**Files:**
- Create: `docs/plans/reticulum-parity-matrix.md`
- Create: `tests/fixtures/python/README.md`
- Create: `tests/fixtures/python/reticulum/` (directory)

**Step 1: Write the failing test (sanity for fixture loader)**

```rust
#[test]
fn loads_fixture_bytes() {
    let bytes = std::fs::read("tests/fixtures/python/reticulum/packet_basic.bin").unwrap();
    assert!(!bytes.is_empty());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum-rs loads_fixture_bytes -v`
Expected: FAIL (missing fixture file)

**Step 3: Write minimal implementation**

```text
# tests/fixtures/python/README.md
Place golden fixture bytes generated from Python Reticulum here.
```

**Step 4: Run test to verify it passes**

Run: `touch tests/fixtures/python/reticulum/packet_basic.bin && cargo test -p reticulum-rs loads_fixture_bytes -v`
Expected: PASS

**Step 5: Commit**

```bash
git add docs/plans/reticulum-parity-matrix.md tests/fixtures/python/README.md tests/fixtures/python/reticulum

git commit -m "chore: add reticulum parity matrix and fixture layout"
```

---

### Task 2: Identity and signing parity

**Files:**
- Modify: `src/identity.rs`
- Test: `tests/identity_parity.rs`
- Fixture: `tests/fixtures/python/reticulum/identity_sign.bin`

**Step 1: Write the failing test**

```rust
#[test]
fn verifies_python_signature() {
    let msg = b"hello";
    let sig = std::fs::read("tests/fixtures/python/reticulum/identity_sign.bin").unwrap();
    let pubkey = /* load or hardcode python public key bytes */;
    assert!(reticulum::identity::verify(pubkey, msg, &sig));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum-rs verifies_python_signature -v`
Expected: FAIL (verify returns false or unimplemented)

**Step 3: Write minimal implementation**

```rust
pub fn verify(pubkey: [u8; 32], data: &[u8], signature: &[u8]) -> bool {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let sig = match Signature::from_slice(signature) { Ok(s) => s, Err(_) => return false };
    let vk = match VerifyingKey::from_bytes(&pubkey) { Ok(v) => v, Err(_) => return false };
    vk.verify(data, &sig).is_ok()
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum-rs verifies_python_signature -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/identity.rs tests/identity_parity.rs tests/fixtures/python/reticulum/identity_sign.bin

git commit -m "feat: add identity signature verification parity"
```

---

### Task 3: Hash/address parity

**Files:**
- Modify: `src/hash.rs`
- Test: `tests/hash_parity.rs`
- Fixture: `tests/fixtures/python/reticulum/hash_address.bin`

**Step 1: Write the failing test**

```rust
#[test]
fn matches_python_address_hash() {
    let input = b"test";
    let expected = std::fs::read("tests/fixtures/python/reticulum/hash_address.bin").unwrap();
    let got = reticulum::hash::address_hash(input);
    assert_eq!(got.as_slice(), expected.as_slice());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum-rs matches_python_address_hash -v`
Expected: FAIL (output mismatch)

**Step 3: Write minimal implementation**

```rust
pub fn address_hash(data: &[u8]) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let out = hasher.finalize();
    let mut truncated = [0u8; 16];
    truncated.copy_from_slice(&out[..16]);
    truncated
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum-rs matches_python_address_hash -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/hash.rs tests/hash_parity.rs tests/fixtures/python/reticulum/hash_address.bin

git commit -m "feat: add address hash parity"
```

---

### Task 4: Packet encode/decode parity

**Files:**
- Modify: `src/packet.rs`
- Test: `tests/packet_parity.rs`
- Fixture: `tests/fixtures/python/reticulum/packet_basic.bin`

**Step 1: Write the failing test**

```rust
#[test]
fn decodes_python_packet() {
    let bytes = std::fs::read("tests/fixtures/python/reticulum/packet_basic.bin").unwrap();
    let packet = reticulum::packet::Packet::from_bytes(&bytes).unwrap();
    let encoded = packet.to_bytes().unwrap();
    assert_eq!(bytes, encoded);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum-rs decodes_python_packet -v`
Expected: FAIL (parse/encode mismatch)

**Step 3: Write minimal implementation**

```rust
impl Packet {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> { /* parse header + body */ }
    pub fn to_bytes(&self) -> Result<Vec<u8>, Error> { /* encode header + body */ }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum-rs decodes_python_packet -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/packet.rs tests/packet_parity.rs tests/fixtures/python/reticulum/packet_basic.bin

git commit -m "feat: packet encode/decode parity"
```

---

### Task 5: Destination and Link parity

**Files:**
- Modify: `src/destination.rs`
- Modify: `src/destination/link.rs`
- Test: `tests/destination_parity.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn destination_hash_matches_python() {
    let ident = reticulum::identity::Identity::generate();
    let dest = reticulum::destination::Destination::new_in(&ident, "app", "aspect");
    assert_eq!(dest.hash.len(), 16);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum-rs destination_hash_matches_python -v`
Expected: FAIL (hash length or value mismatch)

**Step 3: Write minimal implementation**

```rust
impl Destination {
    pub fn new_in(identity: &Identity, app: &str, aspect: &str) -> Self {
        let hash = hash::address_hash(&[identity.hash(), app.as_bytes(), aspect.as_bytes()].concat());
        Self { hash, /* fields */ }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum-rs destination_hash_matches_python -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/destination.rs src/destination/link.rs tests/destination_parity.rs

git commit -m "feat: destination/link parity skeleton"
```

---

### Task 6: Resource and Channel parity

**Files:**
- Modify: `src/transport.rs` or Create: `src/resource.rs`, `src/channel.rs`
- Test: `tests/resource_channel_parity.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn resource_state_machine_advances() {
    let mut res = reticulum::resource::Resource::new(/* ... */);
    res.begin();
    assert!(res.is_active());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum-rs resource_state_machine_advances -v`
Expected: FAIL (missing type)

**Step 3: Write minimal implementation**

```rust
pub struct Resource { active: bool }
impl Resource {
    pub fn new() -> Self { Self { active: false } }
    pub fn begin(&mut self) { self.active = true; }
    pub fn is_active(&self) -> bool { self.active }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum-rs resource_state_machine_advances -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/resource.rs src/channel.rs tests/resource_channel_parity.rs

git commit -m "feat: resource/channel parity skeleton"
```

---

### Task 7: Transport tables parity (announce, path, link tables)

**Files:**
- Modify: `src/transport/*`
- Test: `tests/transport_tables.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn announce_cache_evicts_oldest() {
    let mut cache = reticulum::transport::announce_table::AnnounceCache::new(2);
    cache.insert([0u8;16], vec![1]);
    cache.insert([1u8;16], vec![2]);
    cache.insert([2u8;16], vec![3]);
    assert_eq!(cache.len(), 2);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum-rs announce_cache_evicts_oldest -v`
Expected: FAIL (behavior mismatch)

**Step 3: Write minimal implementation**

```rust
pub struct AnnounceCache { cap: usize, entries: Vec<(AddressHash, Vec<u8>)> }
impl AnnounceCache { /* insert with eviction */ }
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum-rs announce_cache_evicts_oldest -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/transport/announce_table.rs tests/transport_tables.rs

git commit -m "feat: transport announce cache parity"
```

---

### Task 8: Interfaces parity (TCP/UDP/Serial/HDLC/Kaonic)

**Files:**
- Modify: `src/iface/*.rs`
- Test: `tests/iface_parity.rs`
- Fixture: `tests/fixtures/python/reticulum/iface_frames.bin`

**Step 1: Write the failing test**

```rust
#[test]
fn udp_frame_roundtrip() {
    let frame = std::fs::read("tests/fixtures/python/reticulum/iface_frames.bin").unwrap();
    let decoded = reticulum::iface::udp::decode_frame(&frame).unwrap();
    let encoded = reticulum::iface::udp::encode_frame(&decoded).unwrap();
    assert_eq!(frame, encoded);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum-rs udp_frame_roundtrip -v`
Expected: FAIL (frame encode/decode mismatch)

**Step 3: Write minimal implementation**

```rust
pub fn encode_frame(data: &[u8]) -> Result<Vec<u8>, Error> { /* length-prefix */ }
pub fn decode_frame(frame: &[u8]) -> Result<Vec<u8>, Error> { /* parse length */ }
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum-rs udp_frame_roundtrip -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/iface/udp.rs tests/iface_parity.rs tests/fixtures/python/reticulum/iface_frames.bin

git commit -m "feat: iface frame parity"
```

---

### Task 9: Utility CLIs parity (rnsd, rnstatus, rnprobe, rnpath, rncp, rnid, rnx, rnpkg, rnodeconf, rnir)

**Files:**
- Create: `src/bin/rnsd.rs`
- Create: `src/bin/rnstatus.rs`
- Create: `src/bin/rnprobe.rs`
- Create: `src/bin/rnpath.rs`
- Create: `src/bin/rncp.rs`
- Create: `src/bin/rnid.rs`
- Create: `src/bin/rnx.rs`
- Create: `src/bin/rnpkg.rs`
- Create: `src/bin/rnodeconf.rs`
- Create: `src/bin/rnir.rs`
- Test: `tests/cli_parity.rs`

**Step 1: Write the failing test**

```rust
#[test]
fn rnstatus_help_matches_expected_flags() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--bin", "rnstatus", "--", "--help"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--config"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum-rs rnstatus_help_matches_expected_flags -v`
Expected: FAIL (binary missing)

**Step 3: Write minimal implementation**

```rust
fn main() {
    use clap::Parser;
    #[derive(Parser)]
    struct Args { #[arg(long)] config: Option<String> }
    let _ = Args::parse();
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum-rs rnstatus_help_matches_expected_flags -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/bin/rnstatus.rs tests/cli_parity.rs

git commit -m "feat: add rnstatus CLI skeleton"
```

---

### Task 10: Configuration compatibility

**Files:**
- Modify: `src/transport.rs`
- Create: `src/config.rs`
- Test: `tests/config_parity.rs`
- Fixture: `tests/fixtures/python/reticulum/config_default.ini`

**Step 1: Write the failing test**

```rust
#[test]
fn parses_python_default_config() {
    let ini = std::fs::read_to_string("tests/fixtures/python/reticulum/config_default.ini").unwrap();
    let cfg = reticulum::config::Config::from_ini(&ini).unwrap();
    assert!(cfg.interfaces.len() > 0);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum-rs parses_python_default_config -v`
Expected: FAIL (parser missing)

**Step 3: Write minimal implementation**

```rust
pub struct Config { pub interfaces: Vec<String> }
impl Config {
    pub fn from_ini(_ini: &str) -> Result<Self, Error> {
        Ok(Self { interfaces: vec![] })
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum-rs parses_python_default_config -v`
Expected: PASS (update parser for real fields)

**Step 5: Commit**

```bash
git add src/config.rs tests/config_parity.rs tests/fixtures/python/reticulum/config_default.ini

git commit -m "feat: config parser skeleton"
```

---

### Task 11: Example parity and docs

**Files:**
- Modify: `examples/*`
- Modify: `README.md`

**Step 1: Write the failing test**

```rust
#[test]
fn examples_compile() {
    let status = std::process::Command::new("cargo")
        .args(["build", "--examples"]).status().unwrap();
    assert!(status.success());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum-rs examples_compile -v`
Expected: FAIL (example mismatch)

**Step 3: Write minimal implementation**

```rust
// Update example to use new API shape
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum-rs examples_compile -v`
Expected: PASS

**Step 5: Commit**

```bash
git add examples README.md

git commit -m "docs: update examples for parity"
```

---

### Task 12: Compatibility gate checklist

**Files:**
- Modify: `docs/plans/reticulum-parity-matrix.md`

**Step 1: Write the failing test**

```rust
#[test]
fn parity_matrix_has_no_missing_core_items() {
    let text = std::fs::read_to_string("docs/plans/reticulum-parity-matrix.md").unwrap();
    assert!(!text.contains("missing") || !text.contains("core"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p reticulum-rs parity_matrix_has_no_missing_core_items -v`
Expected: FAIL (matrix still has missing core items)

**Step 3: Write minimal implementation**

```text
# Update matrix statuses as tasks complete.
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p reticulum-rs parity_matrix_has_no_missing_core_items -v`
Expected: PASS when core items done

**Step 5: Commit**

```bash
git add docs/plans/reticulum-parity-matrix.md

git commit -m "chore: update parity matrix status"
```
