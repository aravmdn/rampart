# Agent instructions (scope: this directory and subdirectories)

## Scope and intent
- This `AGENTS.md` applies to the entire repository.
- Rampart is a developer-first security product for AI coding agents.
- Core product thesis: AI coding agents should run with enforced least privilege over filesystem, network, and process execution, with local-first controls for individuals and policy/audit features for teams.

## Product definition
- Primary problem: AI coding agents currently inherit broad user permissions and can expand blast radius across code, secrets, credentials, and network access.
- Initial target users: individual developers and small engineering teams using tools like Claude Code, Codex, Cursor, Copilot, Aider, Goose, and OpenCode.
- Initial market wedge: Windows desktop developers first, with macOS and Linux following after the Windows execution model is solid.
- Product position: a policy and visibility layer around agent execution, not another code-review bot and not a generic LLM firewall.

## Non-goals
- Do not add cloud dependency to the free tier.
- Do not turn this into a generic observability platform.
- Do not build team admin, SSO, SOC2-heavy workflows, or browser dashboards before the local enforcement loop is solid.
- Do not let macOS/Linux reference paths displace the Windows-first product direction.

## Architecture guardrails
- Prefer local-first architecture.
- Enforce security controls outside model prompts. Prefer OS/kernel primitives, wrappers, allowlists, and deterministic checks over prompt instructions.
- Treat the enforcement engine as replaceable. `greywall` remains a reference implementation for macOS/Linux, but Windows-first delivery means the product cannot assume `greywall` is the only or primary runtime.
- Preserve an engine abstraction so the repo can later support alternate runtimes or a maintained fork.
- The first valuable loop is:
  1. choose project + agent + profile
  2. launch sandboxed session
  3. capture violations/events
  4. explain what was blocked and why
  5. allow the user to refine policy safely

## Target repo shape
- Current repo is greenfield. Build toward this layout unless a later decision replaces it:

## Modules / subprojects

| Module | Type | Path | What it owns | How to run | Tests | Docs | AGENTS |
|--------|------|------|--------------|------------|-------|------|--------|
| desktop | tauri | `apps/desktop/` | Local GUI, tray, onboarding, project/profile picker, live session and history views | `pnpm dev` or `npm run dev` from `apps/desktop/` | UI/unit tests from module | `docs/architecture.md` | `apps/desktop/AGENTS.md` |
| daemon | rust | `crates/rampartd/` | Agent launch orchestration, profile resolution, local APIs, process supervision, persistence | `cargo run -p rampartd` | `cargo test -p rampartd` | `docs/architecture.md` | `crates/rampartd/AGENTS.md` |
| policy-core | rust library | `crates/policy-core/` | Policy schema, validation, template handling, profile compilation, event normalization | consumed by workspace crates | `cargo test -p policy-core` | `docs/architecture.md` | `crates/policy-core/AGENTS.md` |
| engine-greywall | rust adapter | `crates/engine-greywall/` | Wrapper around `greywall` binary, binary discovery, version compatibility, stdout/stderr/event parsing, capability reporting | consumed by daemon | `cargo test -p engine-greywall` | `docs/architecture.md` | `crates/engine-greywall/AGENTS.md` |
| shared-ui | typescript package | `packages/shared-ui/` | Reusable UI primitives, design tokens, session and policy components | module-local story/test flow | package tests | `docs/architecture.md` | `packages/shared-ui/AGENTS.md` |
| docs | docs | `docs/` | Architecture, roadmap, threat model, product decisions, onboarding | n/a | n/a | self | `docs/AGENTS.md` |

- If the real structure diverges, update this file first.

## Priority roadmap
- Phase 0: repo scaffolding, architecture docs, threat model, capability boundaries, product language.
- Phase 1: local desktop shell, project picker, agent/profile picker, launch sandboxed session, event stream UI.
- Phase 2: profile editing, reusable presets, local session history, violation explanations, safe policy refinement.
- Phase 3: team features behind clear boundaries: shared policies, signed profile distribution, centralized audit sync, org settings.
- Phase 4: CI and headless modes, broader engine support, enterprise controls where justified.

## Cross-domain workflows
- Desktop -> daemon:
  - Desktop should talk to the local daemon over a narrow internal API.
  - Keep command construction, process launch, and policy enforcement out of the UI layer.
- Daemon -> engine adapter:
  - Engine adapters own binary discovery, compatibility checks, profile translation, capability reporting, and structured event conversion.
  - Adapters must fail loudly with actionable diagnostics.
- Policy flow:
  - Policies start from opinionated presets per agent/tool/project type.
  - User edits produce validated policy definitions.
  - Daemon compiles validated policy into engine-specific launch configuration.
- Event flow:
  - Engine emits logs, violations, and raw events.
  - Adapter normalizes them into Rampart event types.
  - Daemon persists local history and streams live updates to the desktop app.

## Core domain concepts
- Agent: the AI tool being launched, such as Claude Code or Codex.
- Session: one sandboxed run of an agent against a project/workspace.
- Policy: the user-facing rule set defining allowed paths, network rules, and execution boundaries.
- Profile: a reusable policy preset for a tool or workflow.
- Violation: an attempted action blocked or flagged by enforcement.
- Audit event: structured record of an allow, block, launch, exit, alert, or policy change.
- Capability snapshot: the detected set of enforcement features available on the current engine and OS.

## Security and trust rules
- Default-deny is the baseline posture.
- Explicitly separate:
  - enforcement
  - observability
  - policy authoring
- Treat the repository as public-facing. Do not expose private strategy, personal goals, internal planning notes, unpublished operational details, private markdown files, secrets, credentials, local-only paths, or hidden collaboration context in code, docs, commits, PR text, issues, or generated assets.
- Public artifacts should describe the product, not the founder's private working process.
- Never imply protection that is not actually enforced on the current OS and engine.
- If macOS and Linux capabilities differ, surface that difference in both product behavior and docs.
- If Windows, macOS, and Linux capabilities differ, surface that difference in product behavior and public docs without disclosing internal implementation shortcuts or private roadmap reasoning.
- Be precise about limitations around temporary files, atomic writes, rename semantics, proxying, and engine coverage.
- Prefer open formats and auditable logic. Users must be able to understand why the product blocked something.

## Documentation rules
- Keep architecture and product reasoning in `docs/`.
- Record key product decisions as ADRs once implementation begins.
- When making architecture changes, update `docs/architecture.md` and this file together.
- Keep business model, packaging, and scalability decisions in `docs/business-model.md`.
- When the repository is updated in a way that changes product direction, architecture, module boundaries, workflow, or developer-facing setup, update `AGENTS.md` and `README.md` in the same body of work.
- Keep public documentation official in tone. Do not publish internal execution notes, personal planning details, private prompts, or tactical build instructions unless the user explicitly wants them public.
- Do not create deep docs sprawl early; prefer a few high-signal docs.

## Verification guidance
- Before claiming a feature works, verify the exact loop that matters:
  - launch
  - enforcement
  - event capture
  - user-visible explanation
- Prefer focused module tests over broad noisy runs.
- For sandbox behavior, include at least one negative-path test or manual repro for blocked access.

## Product strategy context
- The strongest near-term wedge is agent blast-radius control for developers and small teams, starting on Windows where agent adoption and enterprise desktop presence are high.
- The defensible product is not just a sandbox binary; it is usable policy management, visibility, trust, and workflow fit.
- The open-source local product is the adoption layer. The primary paid layer is the hosted team control plane: aggregated audit visibility, hosted policy coordination, alerts, and enterprise-ready controls.
- The fastest route to usefulness is local desktop UX over real enforcement primitives, with Windows support treated as a first-class design constraint.

## Git workflow
- Always commit completed changes unless the user explicitly asks not to.
- Always push committed changes unless the user explicitly asks not to or remote state makes push unsafe.
- Use a separate branch for major tasks, multi-file features, or risky refactors.
- Keep `main` stable. Merge major-task branches back to `main` only after verification is complete.
- For small documentation or low-risk maintenance work, direct work on the current branch is acceptable unless the user asks for a separate branch.
- Do not merge unverified work into `main`.
- If branch strategy is unclear, prefer creating a branch rather than committing complex work directly to `main`.

## Do not
- Do not broaden scope into generic AI governance.
- Do not hide unsupported platform gaps.
- Do not ship security theater phrasing.
- Do not couple product messaging to a single upstream engine vendor.
- Do not build enterprise-only features before the local product is compelling.
