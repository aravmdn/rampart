# Rampart

Local-first blast-radius control for AI coding agents.

Rampart is a Windows-first security product for developers who want to run AI coding agents with enforced least privilege over filesystem, network, and process execution. It wraps agent execution in real controls, captures what happened during a session, and explains what was blocked and why.

## Overview

- [Why Rampart](#why-rampart)
- [What It Does](#what-it-does)
- [How It Works](#how-it-works)
- [Principles](#principles)
- [Current Status](#current-status)
- [Getting Started](#getting-started)
- [Repository Layout](#repository-layout)
- [Documentation](#documentation)

## Why Rampart

AI coding agents usually inherit the full permissions of the developer session that launched them. In practice, that can mean reading secrets, writing outside the intended project scope, making outbound network requests, or leaving behind weak visibility into what they actually attempted.

Rampart exists to reduce that blast radius with deterministic controls at the execution layer, not prompt-only instructions. The goal is straightforward: let developers use coding agents without giving them unconstrained access to the machine they are running on.

## What It Does

Rampart is being built around a simple local loop:

1. Choose a project, agent, and profile.
2. Launch a sandboxed session.
3. Capture violations and execution events.
4. Explain what was blocked, allowed, or limited.
5. Let the user refine policy safely for the next run.

The product is intended to make agent security usable, not just technically possible. That means policy should feel understandable at the product level, event history should be useful without digging through raw logs, and platform limitations should be surfaced clearly instead of hidden behind vague claims.

## How It Works

Rampart is structured as a local desktop product with explicit boundaries:

- A desktop app for launch flow, visibility, session history, and policy UX
- A local daemon for orchestration, process supervision, persistence, and internal APIs
- A policy core for schema validation, templates, compilation, and event normalization
- An engine adapter layer so enforcement is not coupled to a single runtime

The current architecture keeps enforcement outside the UI and outside model prompts. Users interact with projects, profiles, violations, and policies. The daemon and engine layers handle command construction, runtime configuration, capability detection, and structured event conversion.

## Principles

- Local-first by default
- Default-deny baseline posture
- Enforcement outside model prompts
- Replaceable enforcement engine
- Honest capability reporting across platforms
- Developer-first UX before team administration

Rampart is not another code review bot, not a generic observability platform, and not a prompt firewall. The product position is a policy and visibility layer around agent execution.

## Current Status

Rampart is early, but the intended product shape is already defined:

- Windows is the primary v1 platform and the main product design constraint
- The desktop shell is built with Tauri
- Local orchestration and policy services are implemented in Rust
- `greywall` remains a reference adapter for macOS and Linux paths, not the permanent product assumption
- The free local product should remain usable without a required cloud dependency

Capability differences matter and will be surfaced directly in product behavior and docs. Rampart should never imply protections that are not actually enforced on the current OS and engine.

## Getting Started

Rampart currently has a real workspace skeleton for the documented modules.

### Prerequisites

- Node.js
- pnpm
- Rust via `rustup`
- Visual Studio C++ build tools for the Rust MSVC toolchain on Windows

### Install dependencies

```bash
pnpm install
```

### Run the desktop app

```bash
pnpm dev:desktop
```

### Build the desktop app

```bash
pnpm build:desktop
```

### Build the shared UI package

```bash
pnpm build:shared-ui
```

The desktop wrapper script adds the default `rustup` cargo path if the shell did not inherit it. Native Tauri builds still require a working MSVC link environment.

## Repository Layout

```text
rampart/
|- apps/
|  `- desktop/
|     `- src-tauri/
|- crates/
|  |- rampartd/
|  |- policy-core/
|  `- engine-greywall/
|- packages/
|  `- shared-ui/
`- docs/
```

## Documentation

- [Architecture](/C:/projects/rampart/docs/architecture.md)
- [Threat model](/C:/projects/rampart/docs/threat-model.md)
- [Roadmap](/C:/projects/rampart/docs/roadmap.md)
- [Business model and scalability](/C:/projects/rampart/docs/business-model.md)
- [Workspace ADR](/C:/projects/rampart/docs/adr/0001-workspace-layout.md)
- [Product reference document](/C:/projects/rampart/docs/PRD.md)

## Open Source

Rampart is open source under the Apache 2.0 license. The local product is the adoption layer. The likely paid layer is hosted team functionality such as aggregated audit visibility, policy coordination, alerts, and enterprise-ready controls, rather than charging for the local sandbox itself.

## License

Apache 2.0. See [LICENSE](/C:/projects/rampart/LICENSE).
