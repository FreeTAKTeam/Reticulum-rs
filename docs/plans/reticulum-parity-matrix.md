# Reticulum Parity Matrix

Status legend: missing | partial | done

| Python Module | Rust Module | Status | Tests | Dependencies |
| --- | --- | --- | --- | --- |
| RNS/Reticulum.py | src/transport.rs | missing | - | core primitives |
| RNS/Identity.py | src/identity.rs | partial | tests/identity_parity.rs | crypto, hash |
| RNS/Packet.py | src/packet.rs | missing | tests/packet_parity.rs | hash, identity |
| RNS/Destination.py | src/destination.rs | missing | tests/destination_parity.rs | hash, identity |
| RNS/Link.py | src/destination/link.rs | missing | tests/destination_parity.rs | destination |
| RNS/Resource.py | src/transport.rs or src/resource.rs | missing | tests/resource_channel_parity.rs | transport |
| RNS/Channel.py | src/transport.rs or src/channel.rs | missing | tests/resource_channel_parity.rs | transport |
| RNS/Transport.py | src/transport.rs + src/transport/* | partial | tests/transport_tables.rs | packet, link |
| RNS/Buffer.py | src/buffer.rs | partial | - | - |
| RNS/Discovery.py | src/transport.rs or src/discovery.rs | missing | - | transport |
| RNS/Resolver.py | src/transport.rs or src/resolver.rs | missing | - | transport |
| RNS/Interfaces/*.py | src/iface/*.rs | partial | tests/iface_parity.rs | transport |
| RNS/Cryptography/*.py | src/crypt.rs + src/crypt/* | partial | tests/identity_parity.rs | crypto |
| RNS/Utilities/*.py | src/bin/* | missing | tests/cli_parity.rs | config |
