# Roadmap

Rampart should ship in phases that protect the local enforcement loop first.

## Phase 1

- Desktop shell
- Project picker
- Agent picker
- Profile picker
- Launch and live session visibility

Reference-driven priorities inside this phase:

- Add agent-specific launch adapters instead of treating every agent as a bare executable.
- Add launch preflight diagnostics so Rampart can explain startup failures before a session begins.
- Split launcher flow from active session console flow so the product opens into the right state quickly.
- Surface capability warnings and unsupported Windows coverage before launch, not after a block.
- Preserve terminal-first handoff instead of pulling early interaction into a replacement Rampart chat UI.

## Phase 2

- Profile editing
- Reusable presets
- Local history
- Violation explanations
- Safe policy refinement

Reference-driven priorities inside this phase:

- Persist full session records: launch context, capability snapshot, audit trail, and violations together.
- Add rule-linked violation explanations in product language, including platform limitation notes.
- Add agent-aware profile presets and preflight checks keyed to supported terminal agents.
- Define a stable event taxonomy for launch, allow, block, alert, stop, and policy-change records.
- Treat resume and history as first-class local data, not transient UI state.

## Phase 3 — Windows enforcement engine (complete)

MVP gate closed. Enforcement code is shipped and runtime-validated on Windows.

- ✓ Process containment: Job Objects with `KILL_ON_JOB_CLOSE` contain the agent process tree — runtime verified
- ✓ Network enforcement: WFP per-app-ID outbound BLOCK filters on IPv4 + IPv6, auto-cleanup on session end — runtime verified
- ✓ Filesystem write scoping: Low Integrity token + project root SACL patch — runtime verified
- ✓ Audit trail: ETW provider emitting all session and audit events — runtime verified
- ✓ WSL2 isolation mode: stronger enforcement via Linux VM for users with WSL2 installed
- ✓ Runtime end-to-end validation: enforcement mechanisms tested on Windows with admin rights

Known limitation: OS-level blocks (WFP, Job, SACL) do not stream back as violation events to the session console in Windows Native mode. Blocks fire correctly; UI feedback is silent. WSL2 mode does surface events. Candidate for Phase 4 follow-up.

Reference-driven priorities inside this phase:

- Use OS-native enforcement primitives. Do not use fragile in-process hooking approaches.
- Scope enforcement to the agent process, not machine-wide rules.
- Surface enforcement capability gaps honestly in preflight.
- Wire enforcement events into the existing audit taxonomy without inventing a separate format.
- Leave a clean seam for stronger isolation modes in later phases.

## Phase 4 — Team and distribution features

- Shared policies and signed profile distribution
- Centralized audit sync
- Org settings behind clear team/individual boundaries

Reference-driven priorities inside this phase:

- Keep team controls above the local enforcement loop instead of moving core policy decisions into a hosted plane.
- Reuse the local event model and profile schema for sync features instead of inventing separate admin-only formats.
- Treat remote or bridge workflows as access patterns layered on top of local execution, not as a separate product core.

## Phase 5 — Headless, CI, and broader engine support

- Headless `rampart run -- <agent command>` path reusing desktop launch adapters and compiled profiles
- Per-agent capability matrices and compatibility checks
- AppContainer isolation mode for stronger process-level sandboxing
- Enterprise controls where justified

Reference-driven priorities inside this phase:

- Add headless before enterprise. A CLI path that reuses the daemon's enforcement stack is more valuable earlier than SSO or governance dashboards.
- Add per-agent capability matrices so broader agent support does not weaken the enforcement contract.
- Consider plugin and extension points only after Windows enforcement, explanation, and distribution loops are trustworthy.

## What Changed From Research Review

External implementation review does not change Rampart's product position, but it does make several next steps more concrete.

- Real terminal agents need explicit startup orchestration, so Rampart should invest early in agent adapters and preflight checks.
- Permission and mode state become central quickly, so Rampart should mature its profile and audit model before building team administration.
- Session boundaries matter, so launcher, live session, history, and later remote attachment should be modeled as distinct product states.
- Extension seams matter, but plugins, bridge modes, and broader orchestration should remain downstream of the trustworthy local enforcement loop.
