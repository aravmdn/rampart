# Claude Code instructions (scope: this repository)

This file is Claude Code-specific. Read `AGENTS.md` first — it defines product direction,
architecture guardrails, domain concepts, module layout, git workflow, and documentation
rules that apply to all agents including Claude Code. This file adds context and constraints
specific to working with Claude Code in this repo.

## Environment

- Platform: Windows 11, bash shell via Claude Code
- Rust toolchain: installed via rustup, MSVC target (x86_64-pc-windows-msvc)
- MSVC linker (`msvcrt.lib`) is not available in the bash session Claude Code runs in.
  `cargo build`, `cargo test`, and `cargo check` will fail with LNK1104. Do not retry
  them or treat this as a code error. Rust correctness must be verified by reading and
  reasoning about the code rather than compiling.
- TypeScript tests run fine. Use `pnpm --filter @rampart/desktop exec vitest run` to
  run the desktop test suite. This is the primary verification loop available.
- `pnpm install` must be run before tests if `node_modules` is missing (worktrees start
  clean). Check for `node_modules` before running tests.

## Working style

- Be direct and concise. Do not summarise what you just did at the end of a response.
  The diff is visible; a trailing summary adds noise.
- Lead with the action, not the reasoning. Skip preamble.
- Use the TodoWrite tool for multi-step tasks so progress is visible.
- Read files before editing them. Do not propose changes based on memory of a file read
  earlier in the conversation if the file could have changed.
- Prefer editing existing files to creating new ones.
- Do not add comments, docstrings, or type annotations to code you did not touch.

## Rust

- Since `cargo check` is unavailable in the bash environment, reason carefully about
  Rust code before writing it. Check trait bounds, lifetimes, and `use` imports manually.
- When adding new public items to a crate, verify they are imported at the call site.
- When adding new Tauri commands, register them in `tauri::generate_handler![]` in
  `apps/desktop/src-tauri/src/main.rs`.
- `thiserror` v2 is in use. Error derive syntax is v2-compatible.
- Serde: types that cross the Tauri IPC boundary must derive `Serialize`/`Deserialize`.
  Rust uses `snake_case` fields; the TypeScript mapping layer in `tauriDaemonClient.ts`
  converts to `camelCase`.

## TypeScript / frontend

- The Tauri IPC mapping layer lives in `apps/desktop/src/daemon/tauriDaemonClient.ts`.
  When adding new Rust-side fields, update the raw type, the mapping function, and the
  TypeScript contract type in `contracts.ts` together.
- The mock client in `mockDaemonClient.ts` must implement every method in `DaemonApi`.
  Keep it in sync when the contract changes.
- The `shared-ui` package exports components and types consumed by the desktop app.
  Imports resolve via the `@rampart/shared-ui` alias defined in `vite.config.ts`.
- Test environment is jsdom. Tests run via vitest. The `App.test.tsx` integration test
  covers the full launch flow and is the primary regression check.

## Git and branching

- This repo uses worktrees for Claude Code sessions. The current worktree branch is
  `claude/quirky-raman`. Changes are pushed to `main` via fast-forward after each
  completed task body.
- Commit after every finished task. Push immediately unless asked otherwise.
- Commit messages: short imperative subject, body explaining why not what, Co-Authored-By
  trailer for Claude.
- Do not commit `node_modules`, `.rampart/` local state, or any file matching `.gitignore`.

## Module responsibilities (quick reference)

| Module | Path | Owns |
|--------|------|------|
| desktop frontend | `apps/desktop/src/` | React shell, pickers, session console, IPC client |
| Tauri backend | `apps/desktop/src-tauri/src/main.rs` | Tauri commands, state management |
| daemon | `crates/rampartd/src/lib.rs` | Service, adapters, preflight, persistence, process management |
| policy-core | `crates/policy-core/src/lib.rs` | Types, validation, compilation, presets |
| engine-greywall | `crates/engine-greywall/src/lib.rs` | Engine trait, capability snapshot, event normalization |
| shared-ui | `packages/shared-ui/src/` | Reusable React components and types |

## What is done (do not re-implement)

Phase 1 Early priorities are complete. Do not rewrite or re-add:

- `AgentAdapter` struct and per-agent adapter logic in rampartd
- `run_preflight()` and `PreflightReport` in rampartd
- `flatten_capabilities()` and `FlatCapabilityItem` in rampartd
- `preflight_check` Tauri command
- Launcher / session console split in `App.tsx`
- Capability warning panel in the launcher view
- Terminal-first handoff note in the launcher view

Current focus is Late Phase 1 / Early Phase 2:
- Full session records (launch context + capability snapshot + events + violations together)
- Rule-linked violation explanations with platform limitation notes
- Stable audit event taxonomy across engines and agents
