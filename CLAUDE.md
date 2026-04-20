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
- A `PreToolUse` hook in `.claude/settings.json` automatically runs `git pull --ff-only`
  before every `Edit`/`Write`/`NotebookEdit` call to stay in sync with GitHub.
- Commit after every finished task. Push immediately after committing — do not leave
  committed changes unpushed.
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

Phases 1 and 2 are complete. Do not rewrite or re-add:

- `AgentAdapter` struct and per-agent adapter logic in rampartd
- `run_preflight()` and `PreflightReport` in rampartd
- `flatten_capabilities()` and `FlatCapabilityItem` in rampartd
- `preflight_check` Tauri command
- Launcher / session console / history / profile-editor four-view split in `App.tsx`
- Capability warning panel in the launcher view
- Terminal-first handoff note in the launcher view
- `SessionHistoryRecord` with capability snapshot in rampartd
- `ViolationExplanation` and `PlatformLimitation` in policy-core
- `AuditEventCategory` and domain-specific `AuditEventKind` variants
- `HistoryList` and `HistoryDetailPanel` in shared-ui
- History detail view in `App.tsx` with "View all history" navigation
- `agent_profile_presets()` in policy-core (ClaudeCode, Codex, Aider presets)
- Agent-filtered `launch_context()` and `profiles_for_agent()` in rampartd
- `ProfileDetail`, `loadProfile`, `saveProfile` in contracts and both daemon clients
- `ProfileEditorPanel` in shared-ui (product-language policy editing with textarea lists and default-action toggles)
- `PolicyEditorView`, `PolicySuggestion` types in shared-ui
- Profile editor view in `App.tsx` with "Edit selected profile" entry from launcher
- Policy refinement flow: "Adjust policy" on violations → editor pre-populated with targeted suggestion

Phase 3 (Windows enforcement engine) — enforcement code complete:
- `WindowsJob`: Job Object process tree containment, wired in `LocalProcessRunner`
- `set_process_low_integrity()`: post-spawn token → Low Integrity (S-1-16-4096)
- `patch_project_low_integrity_label()`: Low SACL on project root so agent can write there
- `WfpNetworkGuard`: per-app-ID outbound BLOCK on ALE V4+V6; dynamic session auto-cleans on Drop
- `EtwAuditProvider`: ETW provider GUID 7E5A6B4C-...; EventWriteString on every AuditEvent
- `IsolationMode` enum in policy-core (WindowsNative / Wsl2); field on Session and LaunchSessionRequest
- `Wsl2Enforcer` in engine-windows: Supported capabilities; used when isolation_mode = Wsl2
- `detect_wsl2()` + WSL2 preflight diagnostic in rampartd
- `win_path_to_wsl()`: C:\... → /mnt/c/... for WSL working directory
- WSL2 launch path in LocalProcessRunner: `wsl --cd <linux_path> -- <cmd>`; Win32 hooks skipped
- `threat-model.md`: honest accidental-overreach framing, gaps, WSL2/AppContainer seam

Remaining for Phase 3 MVP gate:
- End-to-end violation flow tested at runtime (requires Windows + admin rights)
- AppContainer left as a clean seam for Phase 5

## End-of-session checklist

At the end of each work session, check whether any of the following need updates based on what changed:
- `README.md` — current status, completed items, workflow description
- `AGENTS.md` — phase completion markers, current focus, what-is-done list
- `CLAUDE.md` — what-is-done list, current focus
- `docs/architecture.md`, `docs/roadmap.md` — if architecture or roadmap shifted

## Knowledge base — vault + graph (second brain)

The vault (`rampart/`) is my second brain for this project. I own it. I read it, write to it, restructure it, add notes, add cross-links, and keep it current. It is not a static reference — it is an active working context that I build up over time so each session starts with full context without re-deriving it from raw files.

**Rule: the vault is always the first read and the last write in every session.**

### Session protocol

**Start of every session:**
1. Read `08 - Next Session.md` — this is the single cold-start note; it tells me exactly where we left off and what to do next
2. Read `02 - Phase Status.md` if working on features
3. Run `"/c/Users/20243223/AppData/Roaming/Python/Python311/Scripts/graphify.exe" update .` to refresh the AST graph
4. For code-structural questions (what calls what, where is X defined), query: `graphify.exe query "question"`

**End of every session:**
- Update every vault note touched by what changed — not just `02`
- Update `08 - Next Session.md` to reflect the new state so the next session starts instantly
- After code edits: `graphify.exe update .`
- A stale vault note is worse than no note

### Vault ownership rules

- Add new notes whenever a topic grows too large or distinct to stay in an existing note
- Add cross-links between notes when one note references something explained in another
- Restructure notes whenever the current structure makes context harder to find
- `07 - Gotchas and Decisions.md` is where I record non-obvious bugs, IPC quirks, and why specific decisions were made — update it whenever something surprising comes up
- `08 - Next Session.md` is always the single most up-to-date summary of where we are and what's next

### Vault index — `rampart/`

| Note | Purpose | Update when |
|------|---------|-------------|
| `00 - Rampart Overview.md` | product thesis, MVP bar | MVP bar or product thesis changes |
| `01 - Architecture.md` | layers, modules, IPC boundary | new module, boundary change, IPC change |
| `02 - Phase Status.md` | done/current/future phases | phase completes, focus shifts, feature lands |
| `03 - Windows Enforcement Strategy.md` | enforcement decisions, primitives, threat model | strategy decision, primitive ruled in/out |
| `04 - MVP Session Plan.md` | session log, blockers | session completes, blocker changes |
| `05 - Working Rules for Claude.md` | env constraints, style rules, git rules | env changes, new rule established |
| `06 - What Is Done (Do Not Re-Implement).md` | itemized shipped features | any feature ships |
| `07 - Gotchas and Decisions.md` | IPC quirks, known bugs, decision rationale | anything surprising discovered |
| `08 - Next Session.md` | cold-start summary, next tasks, open questions | end of every session |

### Graphify graph — `graphify-out/`

Community 18 = Rampart-specific code. Communities 0–17 = reference corpus (ignore for Rampart questions).

- Code queries: `graphify.exe query "question"` (BFS, 2000-token budget)
- Path queries: `graphify.exe path "NodeA" "NodeB"`
- Refresh after edits: `graphify.exe update .`
- God nodes and community map: `graphify-out/GRAPH_REPORT.md`
