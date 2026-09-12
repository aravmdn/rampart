<div align="center">

# Rampart

**Run AI coding agents with a smaller blast radius.**

Local-first policy, OS-level enforcement, and an audit trail for agent sessions on Windows.

[![Status](https://img.shields.io/badge/status-alpha-f59e0b.svg)](#project-status) [![Platform](https://img.shields.io/badge/platform-Windows_10%2F11-0078d4.svg?logo=windows11&logoColor=white)](#quick-start) [![Stack](https://img.shields.io/badge/stack-Rust_%2B_Tauri_%2B_React-7c3aed.svg)](#architecture) [![License](https://img.shields.io/badge/license-Apache_2.0-2f855a.svg)](LICENSE)

</div>

Rampart launches tools such as Claude Code, Codex, and Aider inside a policy-controlled session. Its
Windows engine contains the process tree, restricts writes to higher-integrity locations, blocks outbound
connections through the Windows Filtering Platform, and records session activity for review —
without replacing the agent's native terminal experience.

> [!IMPORTANT]
> Rampart is an alpha, build-from-source project. It reduces **accidental overreach by well-behaved
> coding agents**; it is not an adversarial sandbox. Windows Native mode contains writes, but is not
> a general read-isolation boundary. Read the [threat model](docs/threat-model.md) before relying on
> it for sensitive workloads.

## Why Rampart

- **Controls outside the prompt.** Operating-system primitives enforce the session boundary.
- **Preflight before launch.** Project, agent, profile, and platform capabilities are checked first.
- **Readable policies.** Profiles describe project paths, network access, and child commands without
  exposing users to raw engine syntax.
- **Visible sessions.** The desktop shows live state, violations, explanations, and local history;
  the CLI emits JSONL for terminal workflows.
- **Local-first.** The desktop workflow requires no account or hosted backend.
- **Replaceable adapters.** Agent-specific launch behavior stays separate from enforcement engines.

## What ships today

| Layer | Capability |
|---|---|
| Process | Windows Job Object containment with `KILL_ON_JOB_CLOSE` |
| Filesystem | Low Integrity token plus a project-root mandatory label; writes are limited by integrity level |
| Network | Per-application WFP outbound blocking on IPv4 and IPv6 |
| Audit | ETW emission, normalized events, violation explanations, and session history |
| Policy | Agent-aware presets, profile editor, signed profiles, and org policy floors |
| Interfaces | Tauri desktop launcher/session console and a headless Rust CLI |

First-class adapters and presets cover Claude Code, Codex, Aider, Cursor, GitHub Copilot CLI, Goose,
OpenCode, and Gemini CLI.

## Quick start

You need Windows 10/11, an Administrator PowerShell, Node.js 20+, pnpm 10, Rust with the MSVC
toolchain, Visual Studio 2022 C++ Build Tools, and at least one supported agent CLI on `PATH`.

~~~powershell
git clone https://github.com/aravmdn/rampart.git
cd rampart
npm install --global pnpm@10
pnpm install
pnpm dev:desktop
~~~

Choose a project, agent, and profile; review preflight; then start the session. Terminal-first agents
open separately while Rampart owns enforcement and state. See
[`GETTING_STARTED.md`](GETTING_STARTED.md) for the full walkthrough and troubleshooting.

## Headless CLI

The CLI uses the same service and policy path as the desktop app:

~~~powershell
cargo run -p rampart-cli -- list-profiles --agent codex --project C:\path\to\project
cargo run -p rampart-cli -- run --agent codex --profile codex.strict --project C:\path\to\project
~~~

Press Ctrl+C to stop the session and release the process Job Object and network filters.

## The session loop

~~~text
choose project + agent + profile
              |
              v
      capability preflight
              |
              v
     launch contained session
              |
              v
  observe events and blocked actions
              |
              v
   review history / refine policy
~~~

Rampart is a launch-and-control shell, not another agent chat client. The coding tool keeps its
normal interface while Rampart manages the boundary around it.

## Security boundary

Windows Native mode targets three practical failure classes:

1. **Runaway processes** — the process tree ends when its Job Object closes.
2. **Unintended writes** — the labeled project stays writable while Medium-or-higher integrity
   locations remain protected.
3. **Unexpected outbound connections** — WFP filters block traffic for the resolved agent
   application and are removed with the session.

Rampart does **not** claim to contain malicious processes, kernel exploits, direct-syscall bypasses,
side channels, or privilege escalation. Native mode also does not prevent reads from files the
Windows user could already read. Use a dedicated VM or another hypervisor-backed boundary for
adversarial code or strict secret isolation.

The policy editor can express more than the native engine currently enforces. Native WFP filters
block the resolved application broadly, without translating the profile's host allowlist into
exceptions. An interpreter-based agent can therefore affect other processes using that same
interpreter path. Job Objects contain a process tree but do not enforce a child-command allowlist,
and Low Integrity is not an exact project-path allowlist.

Enforcement setup failures currently log diagnostics and can leave a session running with reduced
protection. A successful launch or preflight alone is not proof that every control was installed.
Review runtime diagnostics as well as the [threat model](docs/threat-model.md).

## Architecture

~~~text
Desktop (Tauri + React)           Headless CLI
             \                       /
              +------ rampartd ------+
                      |       |
                      |       +--- policy-core
                      |            schemas, presets, validation, events
                      |
                      +--- engine adapter
                           |-- Windows: Job / Low Integrity / WFP / ETW
                           `-- Greywall reference path
                                      |
                                      v
                                 agent process
~~~

The UI owns the workflow. `rampartd` owns orchestration, supervision, persistence, and internal APIs.
`policy-core` owns the policy and event vocabulary. Engine adapters translate that model into
platform-specific enforcement. See [the architecture guide](docs/architecture.md) for details.

## Project status

Rampart is alpha software and currently installs from source:

- Windows enforcement primitives are implemented; the project records Administrator-level runtime
  checks in its [roadmap](docs/roadmap.md). Native coverage remains limited as described above.
- The WFP violation subscription exists; block-to-console delivery still needs final end-to-end
  confirmation with a real agent.
- The launcher, profile editor, session console, history, and local persistence are implemented.
- There is no signed installer or stable release yet.
- macOS and Linux are not first-class supported platforms; `engine-greywall` is a reference adapter.

See the [roadmap](docs/roadmap.md) and [changelog](CHANGELOG.md) for current delivery details.

## Repository layout

~~~text
apps/desktop/           Tauri + React desktop app
apps/cli/               headless rampart binary
crates/rampartd/         orchestration, persistence, preflight, and APIs
crates/policy-core/      policy schema, presets, validation, and event taxonomy
crates/engine-windows/   Windows enforcement and launch
crates/engine-greywall/  reference adapter for non-Windows paths
packages/shared-ui/      reusable React components and design tokens
docs/                   architecture, threat model, roadmap, and public decisions
~~~

## Development and contributing

~~~powershell
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
pnpm --filter @rampart/desktop test
pnpm --filter @rampart/desktop typecheck
pnpm build
~~~

Read [`CONTRIBUTING.md`](CONTRIBUTING.md) before opening a pull request. Bugs and focused proposals
are welcome in [GitHub Issues](https://github.com/aravmdn/rampart/issues).

## Documentation

| Document | Purpose |
|---|---|
| [Getting started](GETTING_STARTED.md) | Installation, first session, CLI, and troubleshooting |
| [Threat model](docs/threat-model.md) | Security guarantees, exclusions, and limitations |
| [Architecture](docs/architecture.md) | Components, policy flow, and engine adapters |
| [Roadmap](docs/roadmap.md) | Phase status and next delivery work |
| [Contributing](CONTRIBUTING.md) | Development setup and contribution expectations |
| [Changelog](CHANGELOG.md) | Shipped changes |

## License

Rampart is available under the [Apache License 2.0](LICENSE).
