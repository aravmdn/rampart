# AGENTS.md — rampartd

Scope: this directory and its subdirectories. Defers to the root AGENTS.md and CLAUDE.md
for repo-wide conventions.

## What this module owns

Agent launch orchestration, profile resolution with agent-specific filtering, local HTTP APIs (launch, stop, list profiles), process supervision, session history and persistence, preflight checks and capability diagnosis, integration between policy-core and enforcement engines.

## How to build / run

```sh
cargo build -p rampartd
cargo run -p rampartd  # starts HTTP daemon on 127.0.0.1:3000
```

## How to test

```sh
cargo test -p rampartd
```

## Key constraints

- Agent adapter abstraction preserves agent-specific startup logic without leaking into engine abstraction.
- `run_preflight()` and `PreflightReport` must run before session launch; blocks launch on preflight failure.
- Profile resolution via `launch_context()` and `profiles_for_agent()` is agent-aware; filters presets and org policies by agent capability.
- `AgentAdapter` per-agent traits for handoff and session flow.
- Session records carry capability snapshots, full event lists, and violation explanations.
