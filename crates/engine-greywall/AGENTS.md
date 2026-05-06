# AGENTS.md — engine-greywall

Scope: this directory and its subdirectories. Defers to the root AGENTS.md and CLAUDE.md
for repo-wide conventions.

## What this module owns

Wrapper adapter around `greywall` binary; binary discovery and version compatibility; stdout/stderr/event parsing; capability reporting; reference engine implementation for macOS and Linux.

## How to build / run

```sh
cargo build -p engine-greywall
```

## How to test

```sh
cargo test -p engine-greywall
```

## Key constraints

- Engine abstraction (via trait) must be preserved for engine swappability.
- `greywall` binary must be discoverable on PATH; version compatibility checked at runtime.
- Event parsing is fragile and engine-specific; normalize to AuditEventKind in daemon.
- Capability snapshot pulled at session start and carried in SessionHistoryRecord.
- Not primary on Windows; WSL2 mode uses greywall for WSL processes, but Windows Native mode uses WfpNetworkGuard + Job Objects + Low Integrity.
