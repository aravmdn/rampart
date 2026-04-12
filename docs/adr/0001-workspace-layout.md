# ADR 0001: Workspace Layout

## Decision

Use a Rust and TypeScript workspace with separate crates for the daemon, policy core, and engine adapters, plus a desktop app and shared UI package.

## Why

- The desktop app needs a fast UI stack
- The daemon and policy logic need Rust for orchestration and enforcement-adjacent work
- Engine adapters should stay isolated so runtime integration can evolve independently

## Consequences

- Public docs can describe the architecture without exposing private implementation notes
- Each module can evolve with a narrower responsibility
- Public contributors can understand the split between UI, policy, daemon, and engine code
