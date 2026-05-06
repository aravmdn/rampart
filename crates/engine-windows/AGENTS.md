# AGENTS.md — engine-windows

Scope: this directory and its subdirectories. Defers to the root AGENTS.md and CLAUDE.md
for repo-wide conventions.

## What this module owns

Windows-native enforcement: Job Objects for process tree containment, Low Integrity token post-spawn, SACL patch for project root, WFP network guard (per-app-ID outbound BLOCK on IPv4/IPv6 ALE layers), ETW audit provider (Event Trace for Windows) for policy event capture. WSL2 enforcer for Wsl2IsolationMode sessions.

## How to build / run

```sh
cargo build -p engine-windows
```

## How to test

```sh
cargo test -p engine-windows
```

## Key constraints

- Windows-first; MSVC linker required (Visual Studio 2022 Build Tools with Desktop development workload).
- Job Objects, Low Integrity, and WFP must be stacked for defense-in-depth.
- SACL patch on project root allows Low Integrity process writes; without it, write-access is blocked.
- WFP violations blocked at OS level; events do not stream back to session console in Windows Native mode (known limitation).
- WSL2 mode detected via `detect_wsl2()`; `Wsl2Enforcer` activated; greywall used for Linux-side enforcement; `win_path_to_wsl()` converts paths.
- AppContainer seam left open for Phase 5; not activated in MVP.
