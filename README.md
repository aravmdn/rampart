# Rampart

Local-first blast-radius control for AI coding agents.

Rampart is a Windows-first security product for developers who want to run AI coding agents with enforced least privilege over filesystem, network, and process execution. It wraps agent execution in real controls, captures what happened during a session, and explains what was blocked and why.

The intended desktop product should feel like a local launch-and-control console for agent sessions. A user picks a project, chooses an installed agent such as Claude Code or Codex, selects a profile, launches the session through Rampart, and then watches live session state, blocked actions, and history. Rampart does not need to replace every agent's native interface on day one in order to be useful.

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-1f2937.svg)](./LICENSE)
[![Platform: Windows First](https://img.shields.io/badge/platform-Windows%20first-0f766e.svg)](#current-status)
[![Architecture: Local First](https://img.shields.io/badge/architecture-local%20first-1d4ed8.svg)](#principles)
[![Status: Early](https://img.shields.io/badge/status-early-c2410c.svg)](#current-status)

## Overview

- [Why Rampart](#why-rampart)
- [What It Does](#what-it-does)
- [How It Works](#how-it-works)
- [Execution Model](#execution-model)
- [Principles](#principles)
- [Current Status](#current-status)
- [Getting Started](#getting-started)
- [Repository Layout](#repository-layout)
- [Documentation](#documentation)
- [Public Documentation Policy](#public-documentation-policy)

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

## Expected User Workflow

The current intended desktop workflow is:

1. Open Rampart.
2. Choose a local project or repository.
3. Choose an installed agent, such as Claude Code, Codex, Aider, Goose, or OpenCode.
4. Accept or adjust a suggested policy profile.
5. Launch the agent session through Rampart.
6. Watch live session state, blocked actions, and explanations.
7. Stop the session and review local history if needed.

The first product experience should feel closer to a safe launcher and session console than to a generic security dashboard.

## How It Works

Rampart is structured as a local desktop product with explicit boundaries:

- A desktop app for launch flow, visibility, session history, and policy UX
- A local daemon for orchestration, process supervision, persistence, and internal APIs
- A policy core for schema validation, templates, compilation, and event normalization
- An engine adapter layer so enforcement is not coupled to a single runtime

The current architecture keeps enforcement outside the UI and outside model prompts. Users interact with projects, profiles, violations, and policies. The daemon and engine layers handle command construction, runtime configuration, capability detection, and structured event conversion.

Rampart should also support terminal-first users over time. The desktop app is the clearest entry point, but the product direction includes a local CLI and headless execution path so developers who normally work in terminals can still run `claude`, `codex`, or similar tools through Rampart-managed profiles and enforcement.

## Execution Model

```text
Choose project + agent + profile
            |
            v
Launch sandboxed session
            |
            v
Capture allows, blocks, and violations
            |
            v
Explain what happened and why
            |
            v
Refine policy for the next run
```

This is the core product loop Rampart is optimizing for. The goal is not just to block the wrong thing once. The goal is to give developers a repeatable way to run agents safely, understand enforcement outcomes, and improve policy without dropping into raw sandbox internals.

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

Near-term emphasis is intentionally narrow:

- Make the local enforcement loop trustworthy
- Make violations understandable in seconds
- Keep policy authoring above raw engine syntax
- Avoid premature expansion into team admin or generic governance tooling

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

- [Architecture](docs/architecture.md)
- [Threat model](docs/threat-model.md)
- [Roadmap](docs/roadmap.md)
- [Business model](docs/business-model.md)
- [Workspace ADR](docs/adr/0001-workspace-layout.md)
- [Public documentation policy](docs/public-docs-policy.md)

## Public Documentation Policy

The public repo intentionally keeps a small docs set in git so contributors can understand Rampart's architecture and security model without exposing private working notes.

- Public docs live under `docs/` and cover product-level architecture, threat model, roadmap, and business context.
- Private operator notes, prompt files, task notes, and personal instructions are not committed to the public repo.
- Secrets, keys, passwords, tokens, and local-only artifacts must never be tracked.

## Open Source

Rampart is open source under the Apache 2.0 license. The local product is the adoption layer. The likely paid layer is hosted team functionality such as aggregated audit visibility, policy coordination, alerts, and enterprise-ready controls, rather than charging for the local sandbox itself.

## License

Apache 2.0. See [LICENSE](/C:/projects/rampart/LICENSE).
