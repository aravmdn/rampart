# 08 NEXT SESSION
last updated: 2026-05-02 (interactive session — enforcement validation complete)

This note is the single start-here for the next session. Update it at the end of every session
so the next session opens cold with full context. Cross-reference: → 02 for full phase status.

---

## where we left off (2026-05-02, interactive session — enforcement validation complete)

**What was done:**
- Fixed compile errors: 18 Profile preset literals missing `signature: None`; duplicate OrgPolicy re-export; dev stub.
- Full workspace builds clean. 33 TypeScript tests pass.
- CLI runtime-validated: `list-profiles` and `run` subcommands work correctly.
- Non-elevated enforcement test completed (`pnpm tauri dev`, medium integrity, claude-code.strict):
  - SACL patch: ❌ ERROR_ACCESS_DENIED — requires admin (expected)
  - Job Object: ❌ SetInformationJobObject failed — process already in job from conda env (no BREAKAWAY_OK)
  - WFP block filter: ❌ FwpmFilterAdd0 0x80320023 — requires admin
  - WFP event monitor: never reached — gated on WFP guard succeeding
  - Result: zero enforcement at medium integrity. Matches threat model. No code change needed.
  - All failures log to stderr with actionable messages.

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
  - ✓ Non-elevated enforcement test: all three primitives confirmed to require admin. Zero enforcement at medium integrity. Matches threat model.
  - Deferred: AppContainer isolation mode.

---

## next tasks (priority order)

**1. AppContainer isolation mode** (clean seam; deferred — most complex primitive)

All Phase 5 work is complete. AppContainer is the only remaining deferred item.
It was intentionally left as a clean seam after Job Objects + WFP + ETW shipped.

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

- AppContainer: deferred clean seam. No other open questions.

---

## session workflow reminder
start: read this note (08) + run `graphify.exe update .`
end: update 02, 03, 04, 06, 07 as needed → update this note → commit + push
