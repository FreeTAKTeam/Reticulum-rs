
# Reticulum-rs

**Reticulum-rs** is a Rust implementation of the [Reticulum Network Stack](https://reticulum.network/) — a cryptographic, decentralised, and resilient mesh networking protocol designed for communication over any physical layer.

This project is open source and community-owned, focused on bringing Reticulum capabilities to the Rust ecosystem with clear APIs, reproducible behavior, and portable deployment options.

## Features

- Cryptographic mesh networking
- Identity-based trust and routing primitives
- Modular architecture for constrained and general-purpose systems
- Multiple transport options (TCP, serial)
- Example clients for testing and integration

## Structure


```
Reticulum-rs/
├── src/                 # Core Reticulum protocol implementation
│   ├── buffer.rs
│   ├── crypt.rs
│   ├── destination.rs
│   ├── error.rs
│   ├── hash.rs
│   ├── identity.rs
│   ├── iface.rs
│   ├── lib.rs
│   ├── transport.rs
│   └── packet.rs
├── examples/            # Example clients and servers
│   ├── link_client.rs
│   ├── tcp_client.rs
│   ├── tcp_server.rs
│   └── testnet_client.rs
├── Cargo.toml           # Crate configuration
├── LICENSE              # License (MIT/Apache)
└── ...
````
## Getting Started

### Prerequisites

* Rust (edition 2021+)

### Build

```bash
cargo build --release
```

### Run Examples

```bash
# TCP client example
cargo run --example tcp_client
```

## License

This project is licensed under the MIT license.

---

Maintained by FreeTAKTeam and contributors.
