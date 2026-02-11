# ADR-0001: Enterprise Reliability Split

## Status
Accepted

## Context
The project needed tighter architectural boundaries, stronger governance, and removal of proprietary coupling.

## Decision
- Keep repositories separate from LXMF-rs.
- Decompose transport and RPC monoliths into focused modules.
- Introduce explicit interface-driver extension traits.
- Remove proprietary hardware references from active code/docs.

## Consequences
- Breaking internal module paths for contributors.
- Better testability and ownership boundaries.
- Cleaner path for external proprietary integrations without polluting core.
