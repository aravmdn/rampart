# Rampart

Rampart is a Windows-first desktop application that runs AI coding agents under enforced least privilege. It applies OS-level controls — Windows Job Objects, Low Integrity tokens, and WFP network filters — to constrain what an agent can read, write, connect to, and spawn, then surfaces live audit events, blocked actions, and session history in a local GUI and headless CLI.

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-1f2937.svg)](./LICENSE)
[![Platform: Windows](https://img.shields.io/badge/platform-Windows-0f766e.svg)](#architecture)
[![Status: alpha](https://img.shields.io/badge/status-alpha-f59e0b.svg)](docs/roadmap.md)

---

## Quick Start

Prerequisites: Windows 10/11, Administrator shell, Node 20+, pnpm 9, Rust + MSVC build tools, at least one agent CLI (`claude`, `codex`, `aider`, etc.) on PATH.

```powershell
git clone https://github.com/rampart-dev/rampart.git
cd rampart
pnpm install
pnpm dev:desktop   # run from an Administrator PowerShell
```

For the full walkthrough — prerequisites, CLI usage, troubleshooting — see [GETTING_STARTED.md](GETTING_STARTED.md).

---

## Why Rampart

- **Real enforcement, not prompt instructions.** Controls are applied at the OS level via Job Objects, token integrity, and WFP filters. The agent cannot bypass them by ignoring a system prompt.
- **Default-deny baseline.** Agents need explicit allow rules for filesystem paths, network hosts, and allowed child processes. Nothing is permitted by accident.
- **Agent-agnostic.** Supports Claude Code, Codex, Aider, Cursor, Copilot, Goose, OpenCode, and Gemini CLI. Adding a new agent requires an adapter, not a rewrite.
- **Local-first.** No account, no cloud service, no required backend. Session history, profiles, and audit events stay on the machine that ran the session.

---

## What It Does Today

- **Desktop launcher**: project picker, agent picker, profile picker, preflight diagnostics with capability snapshot
- **Session console**: live audit stream (file, network, process events), violation panel with structured explanations, "Adjust policy" flow from blocked actions
- **Profile editor**: filesystem paths, network hosts, allowed commands in product language; no raw engine syntax required
- **Session history**: full per-session record with audit trail, violations, capability snapshot, and stop reason
- **Windows enforcement**: Job Object process tree containment, Low Integrity token + project root SACL, WFP per-app-ID outbound network block, ETW audit trail — all runtime-verified
- **WSL2 isolation mode**: stronger isolation inside a Hyper-V Linux VM when WSL2 is available
- **Signed profiles**: ed25519 signatures; built-in presets are signed; invalid signatures block launch
- **Org policy floor**: `OrgPolicy` struct defines a minimum policy applied on top of local profiles; preflight annotates which diagnostics come from org policy
- **Centralized audit sync**: local outbox drains to a configured HTTPS endpoint on session stop
- **Headless CLI**: `rampart run` applies full enforcement without the GUI; JSONL events to stdout

---

## Architecture

```text
 Desktop (Tauri + React)
        |  Tauri invoke bridge
        v
 rampartd (Rust daemon)
   - session orchestration
   - profile resolution
   - persistence
   - local APIs
        |
        +---> policy-core
        |       - schema validation
        |       - profile compilation
        |       - event normalization
        |
        +---> engine adapter
                - WindowsEnforcer (Job Object, Low Integrity, WFP, ETW)
                - Wsl2Enforcer   (wsl --cd launch, Linux-native enforcement)
                - GreywallAdapter (reference adapter for macOS/Linux paths)
                  |
                  v
              agent process (claude, codex, aider, ...)
```

The desktop shell owns the user workflow. The daemon owns command construction, process launch, policy enforcement, and persistence. Engine adapters are replaceable; adding a new isolation backend requires implementing the engine trait, not touching the daemon or desktop layers.

---

## Repository Layout

```text
rampart/
|- apps/
|  |- desktop/         Tauri + React desktop app
|  |  `- src-tauri/    Tauri backend and Tauri commands
|  `- cli/             Headless rampart binary
|- crates/
|  |- rampartd/        Daemon: session orchestration, persistence, local APIs
|  |- policy-core/     Policy schema, validation, presets, event taxonomy
|  |- engine-greywall/ Reference adapter (macOS/Linux compatibility)
|  `- engine-windows/  Windows enforcement (Job Object, WFP, ETW, WSL2)
|- packages/
|  `- shared-ui/       Reusable React components (HistoryList, ProfileEditorPanel, ...)
`- docs/
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [GETTING_STARTED.md](GETTING_STARTED.md) | Prerequisites, installation, walkthrough, CLI, troubleshooting |
| [docs/architecture.md](docs/architecture.md) | Module boundaries, IPC layer, engine adapter contract |
| [docs/threat-model.md](docs/threat-model.md) | What Rampart contains, what it does not, known gaps |
| [docs/roadmap.md](docs/roadmap.md) | Phase status and what is coming next |
| [docs/business-model.md](docs/business-model.md) | Free local product and paid team layer |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Development setup, tests, CLI build, issue tracker |
| [CHANGELOG.md](CHANGELOG.md) | Release history |

---

## License

Apache 2.0. See [LICENSE](LICENSE).
