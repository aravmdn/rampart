# Rampart

Rampart is a local-first blast-radius limiter for AI coding agents.

Rampart is a developer-first security product for running AI coding agents inside controlled execution boundaries, with clear visibility into attempted access and policy outcomes.

## Overview

Rampart exists to solve an execution-layer problem:
- AI coding agents inherit broad user permissions by default.
- Most teams do not have deterministic controls over what those agents can read, write, or call.
- Existing products skew toward review, gateways, or enterprise governance rather than local enforcement for day-to-day developer workflows.

Rampart is intended for:
- Windows developers first
- macOS and Linux developers second
- individual users and small engineering teams
- local-first enforcement, visibility, and policy control

## Architecture

```text
rampart/
|- AGENTS.md
|- README.md
|- docs/
|  |- architecture.md
|  |- roadmap.md
|  |- threat-model.md
|  `- adr/
|     `- 0001-workspace-layout.md
|- apps/
|  `- desktop/
|     `- AGENTS.md
|- crates/
|  |- rampartd/
|  |  `- AGENTS.md
|  |- policy-core/
|  |  `- AGENTS.md
|  `- engine-greywall/
|     `- AGENTS.md
`- packages/
   `- shared-ui/
      `- AGENTS.md
```

## Intended module boundaries

| Module | Role |
|--------|------|
| `apps/desktop` | Local GUI, tray, project/agent/profile selection, live session view |
| `crates/rampartd` | Local daemon for launch orchestration, supervision, event streaming, persistence |
| `crates/policy-core` | Policy schema, validation, profile logic, event normalization |
| `crates/engine-greywall` | Adapter around the initial enforcement engine |
| `packages/shared-ui` | Shared UI primitives and domain-facing components |
| `docs` | Architecture, roadmap, threat model, ADRs |

## Documentation

- [Root agent instructions](C:\projects\rampart\AGENTS.md)
- [Architecture](C:\projects\rampart\docs\architecture.md)
- [Roadmap](C:\projects\rampart\docs\roadmap.md)
- [Threat model](C:\projects\rampart\docs\threat-model.md)

## Design principles

- Keep the product local-first.
- Keep core usage viable without mandatory cloud dependency.
- Preserve an engine adapter boundary instead of binding the product to one enforcement implementation.
- Treat Windows as the primary target platform.
- Be explicit about platform limitations.
- Focus on execution-layer control rather than generic AI governance.

## License

Apache 2.0. See [LICENSE](C:\projects\rampart\LICENSE).
