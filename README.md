# Rampart

Rampart is a security control plane for AI coding agents.

It is designed to reduce the blast radius of tools such as Claude Code, Codex, Cursor, Copilot, Aider, Goose, and OpenCode by enforcing execution boundaries around what those agents can access and by making policy outcomes visible to the user.

## Why Rampart

AI coding agents operate at the execution layer. They can read files, modify code, invoke tools, and make network requests with the permissions available to the session that launched them.

That creates a practical gap for developers and teams:
- agents often inherit more access than they should
- most workflows lack clear, deterministic policy boundaries
- security visibility usually arrives after the fact, if at all

Rampart exists to close that gap with local-first control, enforcement, and visibility.

## What Rampart Does

Rampart is being built to provide:
- controlled execution boundaries for AI coding agents
- clear policy outcomes for attempted access and blocked actions
- a product workflow that fits day-to-day developer usage rather than only post-hoc review
- a path from individual use to team policy and auditability

## Design Principles

- Local-first by default
- Execution-layer control instead of prompt-only safety
- Replaceable enforcement engine architecture
- Clear communication of platform capabilities and limitations
- Product UX that prioritizes trust, visibility, and operational clarity

## Platform Focus

Rampart is currently oriented around:
- Windows first
- macOS and Linux after the Windows execution model is validated

Platform support should be understood as capability-specific rather than assumed to be identical across operating systems.

## Architecture

Rampart is structured around a small number of clear system boundaries:
- a desktop application for user interaction and live session visibility
- a local daemon for launch orchestration, supervision, and event handling
- a policy core for policy definition, validation, and normalization
- an engine adapter layer so enforcement is not tied to a single runtime

Additional architectural detail is documented in:
- [Architecture](C:\projects\rampart\docs\architecture.md)
- [Threat model](C:\projects\rampart\docs\threat-model.md)
- [Roadmap](C:\projects\rampart\docs\roadmap.md)

## Repository Structure

```text
rampart/
|- apps/
|  `- desktop/
|- crates/
|  |- rampartd/
|  |- policy-core/
|  `- engine-greywall/
|- packages/
|  `- shared-ui/
`- docs/
```

## Documentation

- [Architecture](C:\projects\rampart\docs\architecture.md)
- [Threat model](C:\projects\rampart\docs\threat-model.md)
- [Roadmap](C:\projects\rampart\docs\roadmap.md)
- [Workspace ADR](C:\projects\rampart\docs\adr\0001-workspace-layout.md)

## Open Source

Rampart is open source under the Apache 2.0 license.

The repository is intended to document the product, its architecture, and its public codebase. Public materials are kept focused on the product and its technical boundaries rather than unpublished operational details.

## License

Apache 2.0. See [LICENSE](C:\projects\rampart\LICENSE).
