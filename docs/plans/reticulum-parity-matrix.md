# Reticulum Parity Matrix

Status legend: not-started | partial | done

| Python Module | Rust Module | Status | Tests | Dependencies |
| --- | --- | --- | --- | --- |
| RNS/Reticulum.py | src/transport.rs | not-started | - | core primitives |
| RNS/Identity.py | src/identity.rs | partial | tests/identity_parity.rs | crypto, hash |
| RNS/Packet.py | src/packet.rs | not-started | tests/packet_parity.rs | hash, identity |
| RNS/Destination.py | src/destination.rs | not-started | tests/destination_parity.rs | hash, identity |
| RNS/Link.py | src/destination/link.rs | not-started | tests/destination_parity.rs | destination |
| RNS/Resource.py | src/transport.rs or src/resource.rs | not-started | tests/resource_channel_parity.rs | transport |
| RNS/Channel.py | src/transport.rs or src/channel.rs | not-started | tests/resource_channel_parity.rs | transport |
| RNS/Transport.py | src/transport.rs + src/transport/* | partial | tests/transport_tables.rs | packet, link |
| RNS/Buffer.py | src/buffer.rs | partial | - | - |
| RNS/Discovery.py | src/transport.rs or src/discovery.rs | not-started | - | transport |
| RNS/Resolver.py | src/transport.rs or src/resolver.rs | not-started | - | transport |
| RNS/Interfaces/*.py | src/iface/*.rs | partial | tests/iface_parity.rs | transport |
| RNS/Cryptography/*.py | src/crypt.rs + src/crypt/* | partial | tests/identity_parity.rs | crypto |
| RNS/Utilities/*.py | src/bin/* | not-started | tests/cli_parity.rs | config |
