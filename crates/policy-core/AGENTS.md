# AGENTS.md — policy-core

Scope: this directory and its subdirectories. Defers to the root AGENTS.md and CLAUDE.md
for repo-wide conventions.

## What this module owns

Policy schema (Rule, Ruleset, Profile, OrgPolicy), validation and compilation, template handling, event normalization, violation explanation (ViolationExplanation, PlatformLimitation), agent-aware profile presets (agent_profile_presets), audit event taxonomy (AuditEventCategory, AuditEventKind).

## How to build / run

```sh
cargo build -p policy-core
```

## How to test

```sh
cargo test -p policy-core
```

## Key constraints

- Default-deny baseline: all capabilities blocked by default; policies add explicit allow rules.
- Violations must not imply unsupported protections; ViolationExplanation separates policy reason from platform limitation.
- Agent presets via `agent_profile_presets()` are agent-specific; filtering happens in daemon, not here.
- Event taxonomy is stable across engines; AuditEventKind variants are engine-agnostic where possible.
- OrgPolicy merges via `resolve_effective_policy()`; strict, no fallback; org policy takes precedence over profile.
