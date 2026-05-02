# 08 NEXT SESSION
last updated: 2026-05-02 (scheduled routine #16 — code quality sweep)

This note is the single start-here for the next session. Update it at the end of every session
so the next session opens cold with full context. Cross-reference: → 02 for full phase status.

---

## where we left off (2026-05-02, scheduled routine #16)

Routine #16 was a code quality sweep — no architectural changes.

**What was audited:**
- All `.unwrap()` calls: only one exists, inside a test assert. Clean.
- `todo!`, `unimplemented!`, `unreachable!` macros: none in production paths. Clean.
- Debug `println!`/`eprintln!`: only in intentional diagnostic paths (ETW init, WFP monitor errors, CLI main). Appropriate.
- TypeScript `console.log`/`console.warn` in production: none. Clean.
- DaemonApi interface vs tauriDaemonClient vs mockDaemonClient: all 15 methods fully implemented in all three. Consistent.
- Tauri command registration in main.rs vs DaemonApi: all 15 commands registered. Consistent.
- Rust serde attributes: ProfileDetail + sub-structs all use `#[serde(rename_all = "camelCase")]`. Raw types in tauriDaemonClient.ts correctly use snake_case for non-camelCase structs (AgentCatalogEntry, ProfileSummary, SelectedLaunchConfig). Mapping layer is correct.
- agent_tool_from_id() supports 8 agents: claude-code, codex, cursor, copilot, aider, goose, opencode, gemini.

**One bug fixed:**
- `apps/cli/src/main.rs` usage hint was missing `copilot` from the `--agent` list (7 of 8 listed). Fixed and pushed.

**All 33 TypeScript tests pass.**

Previous routine (#15): doc drift audit — no code changes.

---

## current state

- Phase 1–4 complete.
- Phase 5 progress:
  - ✓ Headless CLI (`apps/cli/`): code written, manually reviewed, not yet compiled.
  - ✓ WFP violation streaming: code written; privilege validation pending runtime test.
  - ✓ WFP monitor error hint: actionable stderr message when access-denied.
  - ✓ Mapping layer: 25 unit tests + 5 App integration tests = 33 total. All DaemonApi methods, all union variants covered.
  - ✓ Per-agent capability matrices: all 8 AgentTool variants have tailored presets.
  - ✓ mockDaemonClient.ts audited: all method signatures match DaemonApi interface exactly.
  - ✓ Docs: architecture.md + README.md fully reflect Phase 5 agent expansion.
  - ✓ CLI usage hint: all 8 agent IDs listed correctly (fixed routine #16).
  - Pending: WFP monitor privilege validation (requires interactive session).
  - Pending: CLI runtime validation (requires VS Dev Shell).
  - Deferred: AppContainer isolation mode.

---

## next tasks (priority order)

**1. WFP monitor privilege validation** (MUST be done in interactive session — NOT schedulable)

Run the desktop app (normal, non-elevated launch), start a session with network blocked,
then check Tauri stderr for:
  - `rampartd: WFP event monitor failed for session '...': ... — run as Administrator`
    → CONFIRMED: document in 07; consider mitigation options.
  - No error line → monitor subscribed successfully; verify blocked connections appear in UI.

**2. CLI runtime validation** (requires VS Dev Shell — NOT schedulable)

- Build: `cargo build --bin rampart --target x86_64-pc-windows-msvc` in VS Dev Shell.
- Test `list-profiles` and `run` subcommands.
- Document findings in `07 - Gotchas and Decisions.md`.

**3. AppContainer isolation mode** (clean seam; deferred — most complex primitive)

- Defer until WFP monitor and CLI are runtime-validated.

**For future scheduled routines:**
- Codebase is clean. No outstanding doc drift, dead code, or TODO items.
- Next schedulable sweep: audit for any new drift introduced by interactive sessions (after WFP/CLI validation).

---

## competitive context (May 2026)
→ Full analysis in `09 - Competitive Landscape.md`

Key points:
- **No competitor has Windows support.** Ash, Safehouse, hazmat are all macOS-only.
- Anthropic shipped Claude Code sandboxing (macOS/Linux only) — validates the problem, doesn't close our moat.
- Microsoft Agent Governance Toolkit (April 2026) targets agent *builders*, not individual devs — different buyer.
- Biggest strategic risk: Anthropic porting their sandbox to Windows. Watch Claude Code releases.
- Biggest differentiator against all Tier 1 (Ash/Safehouse/hazmat): signed profiles, org policy floor, audit sync — already built.

## open questions / blockers

- WFP monitor privilege: likely fails at Medium integrity (unconfirmed until tested).
- CLI not yet compiled / runtime-tested (MSVC linker unavailable in scheduled routine env).

---

## session workflow reminder
start: read this note (08) + run `graphify.exe update .`
end: update 02, 03, 04, 06, 07 as needed → update this note → commit + push
