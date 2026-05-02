# 08 NEXT SESSION
last updated: 2026-05-02 (scheduled routine #21 — clean sweep, no drift found)

This note is the single start-here for the next session. Update it at the end of every session
so the next session opens cold with full context. Cross-reference: → 02 for full phase status.

---

## where we left off (2026-05-02, interactive session — CLI build + validation)

**What was done:**
- Fixed compile errors blocking CLI build: 18 `Profile` preset literals in `policy-core/lib.rs` were
  missing `signature: None` (field added for ProfileSignature but presets not updated).
- Fixed duplicate `pub use policy_core::OrgPolicy` in `rampartd/lib.rs` (conflicted with existing use).
- Fixed `rampartd/src/main.rs` dev stub: same `signature: None` omission.
- Full workspace builds clean (dev profile, MSVC target). 33 TypeScript tests pass.
- CLI validated runtime:
  - `rampart list-profiles --project-root C:\projects\rampart` → returns windows-safe, windows-strict ✓
  - `rampart list-profiles --project-root ... --agent claude-code` → returns claude-code.standard, claude-code.strict ✓
  - `rampart run --agent claude-code --profile claude-code.strict --project C:\projects\rampart` →
    preflight runs, finds claude on PATH, reports enforcement warnings, starts session ✓
- WFP monitor privilege: tested under elevated (admin) session only. Non-elevated test still pending.

Previous routine (#20): clean sweep, same result.

---

## current state

- Phase 1–5 complete (minus AppContainer and non-elevated WFP test).
- Phase 5 status:
  - ✓ Headless CLI (`apps/cli/`): builds and runtime-validated (list-profiles + run).
  - ✓ WFP violation streaming: code written; privilege validation done under admin.
  - ✓ WFP monitor error hint: actionable stderr when access-denied.
  - ✓ Mapping layer: 25 unit tests + 5 App integration tests = 33 total.
  - ✓ Per-agent capability matrices: all 8 AgentTool variants have tailored presets.
  - ✓ mockDaemonClient.ts: all 17 method signatures match DaemonApi.
  - ✓ Docs: architecture.md + README.md fully reflect Phase 5 agent expansion.
  - ✓ Preset literals: all 18 Profile { } blocks have signature: None (fixed this session).
  - ✓ Full workspace (all crates + CLI + desktop backend) compiles clean.
  - Pending: WFP monitor non-elevated test (run desktop app as normal user, verify graceful error hint).
  - Deferred: AppContainer isolation mode.

---

## next tasks (priority order)

**1. WFP monitor non-elevated test** (run desktop app as normal user — NOT schedulable)

Run the Tauri desktop app without elevation. Start a session with network blocked.
Check Tauri stderr:
  - `rampartd: WFP event monitor failed for session '...': ... — run as Administrator`
    → CONFIRMED: document in 07; consider whether to show a UI warning.
  - No error → monitor subscribed at medium integrity (unexpected but possible).

**2. AppContainer isolation mode** (clean seam; deferred — most complex primitive)

- All other Phase 5 work is complete. AppContainer is the remaining deferred item.

**For future scheduled routines:**
- Codebase is clean. No outstanding doc drift, dead code, or TODO items.
- Next schedulable sweep: audit for any new drift introduced by interactive sessions (after WFP/CLI validation).
- Note: routine #19 confirmed the routine #18 fixes held and the full surface is clean. No new work found.

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

- WFP monitor privilege at medium integrity: still unconfirmed (only tested under admin so far).
- AppContainer: deferred clean seam.

---

## session workflow reminder
start: read this note (08) + run `graphify.exe update .`
end: update 02, 03, 04, 06, 07 as needed → update this note → commit + push
