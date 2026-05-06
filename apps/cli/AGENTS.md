# AGENTS.md — cli

Scope: this directory and its subdirectories. Defers to the root AGENTS.md and CLAUDE.md
for repo-wide conventions.

## What this module owns

Headless `rampart` binary with `run` and `list-profiles` subcommands; full enforcement via RampartService; JSONL event stream to stdout; clean Ctrl+C shutdown; primary interface for CI and terminal-first workflows.

## How to build / run

```sh
cargo build --bin rampart
cargo run -p rampart-cli -- run --agent claude-code --profile claude-code.strict --project <path>
cargo run -p rampart-cli -- list-profiles --agent claude-code --project <path>
```

## How to test

CLI testing is covered by integration tests in `crates/rampartd/tests/`. No direct CLI unit tests.

## Key constraints

- Enforcement via RampartService; must not duplicate daemon logic.
- Profile resolution and agent filtering deferred to daemon `launch_context()` and `profiles_for_agent()`.
- Event stream is JSONL to stdout; violations and explanations must match daemon session event schema.
- No cloud dependency; all profiles and policies resolve locally.
