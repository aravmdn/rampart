# Rampart

Rampart is a local-first security control plane for AI coding agents.

It runs supported agents inside an enforced sandbox and gives developers a usable view of what was allowed, what was blocked, and why. The initial product focus is a Windows-first desktop experience, with the enforcement layer kept replaceable so macOS and Linux support can follow without redesigning the product.

## Why It Exists

AI coding agents inherit the permissions of the developer session that launches them. In practice that means an agent may be able to:

- read secrets such as `.env` files, SSH keys, and cloud credentials
- write outside the intended project scope
- make outbound network requests with developer context
- continue acting without a clear audit trail or reliable stop path

Rampart exists to shrink that blast radius with deterministic controls at the execution layer rather than prompt-only instructions.

## What Rampart Does

Rampart is being built around five core capabilities:

- sandboxed session launch for supported coding agents
- project-scoped policy profiles with safe defaults
- live visibility into blocked and allowed activity
- persistent local audit history
- clear user-visible explanations of enforcement outcomes and platform limitations

## Product Principles

- Local-first by default
- Enforcement outside model prompts
- Replaceable engine architecture
- Honest platform capability reporting
- Developer-first UX before team administration

## Current Product Direction

Rampart is currently documented around:

- desktop application built with Tauri
- Rust-based local orchestration and policy services
- a replaceable enforcement layer, with `greywall` retained as a reference adapter for macOS/Linux
- free tier without required cloud dependency
- team features added only after the core local enforcement loop is solid

Current limitations are intentional:

- Windows is the v1 target platform and should have the strongest product workflow coverage
- macOS and Linux may not have identical capability coverage as those paths mature
- network observability and blocking may differ by platform and engine support

## Architecture

Rampart is structured around a few explicit system boundaries:

- desktop app for launch, visibility, and policy UX
- local daemon for orchestration, supervision, and persistence
- policy core for validation and compilation
- engine adapter layer so enforcement is not coupled to a single runtime

Additional detail:

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
- [Product reference document](C:\projects\rampart\docs\PRD.md)

## Open Source

Rampart is open source under the Apache 2.0 license. Public repository materials describe the product, its architecture, and its technical boundaries without exposing private operational context.

## License

Apache 2.0. See [LICENSE](C:\projects\rampart\LICENSE).
