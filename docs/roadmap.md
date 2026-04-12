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

## Phase 3

- Team features behind explicit boundaries
- Shared policies
- Signed profile distribution
- Centralized audit sync

Reference-driven priorities inside this phase:

- Keep team controls above the local enforcement loop instead of moving core policy decisions into a hosted plane.
- Reuse the local event model and profile schema for sync features instead of inventing separate admin-only formats.
- Treat remote or bridge workflows as access patterns layered on top of local execution, not as a separate product core.

## Phase 4

- Headless and CI modes
- Broader engine support
- Enterprise controls where justified

Reference-driven priorities inside this phase:

- Add a headless `rampart run -- <agent command>` path that reuses desktop launch adapters and compiled profiles.
- Add per-agent capability matrices and compatibility checks so broader support does not weaken the core contract.
- Consider plugin and extension points only after Windows launch, enforcement, and explanation loops are trustworthy.

## What Changed From Research Review

External implementation review does not change Rampart's product position, but it does make several next steps more concrete.

- Real terminal agents need explicit startup orchestration, so Rampart should invest early in agent adapters and preflight checks.
- Permission and mode state become central quickly, so Rampart should mature its profile and audit model before building team administration.
- Session boundaries matter, so launcher, live session, history, and later remote attachment should be modeled as distinct product states.
- Extension seams matter, but plugins, bridge modes, and broader orchestration should remain downstream of the trustworthy local enforcement loop.
