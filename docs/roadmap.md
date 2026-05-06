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

MVP gate closed. Enforcement code is shipped and runtime-validated on Windows under admin.

- ✓ Process containment: Job Objects with `KILL_ON_JOB_CLOSE` contain the agent process tree — runtime verified. Note: the initial implementation used `BasicLimitInformation` (class 2); a bug requiring `ExtendedLimitInformation` (class 9) on Windows 11 was surfaced and fixed during this validation run.
- ✓ Network enforcement: WFP per-app-ID outbound BLOCK filters on IPv4 + IPv6, auto-cleanup on session end — runtime verified. Note: a bug where `FwpmFilterAdd0` returned `FWP_E_NULL_DISPLAY_NAME` due to a null `filter.displayData.name` was surfaced and fixed during this validation run.
- ✓ Filesystem write scoping: Low Integrity token + project root SACL patch — runtime verified
- ✓ Audit trail: ETW provider emitting all session and audit events — runtime verified
- ✓ WSL2 isolation mode: stronger enforcement via Linux VM for users with WSL2 installed
- ✓ Runtime end-to-end validation: all five enforcement mechanisms pass under admin on Windows 11

Reference-driven priorities inside this phase:

- Use OS-native enforcement primitives. Do not use fragile in-process hooking approaches.
- Scope enforcement to the agent process, not machine-wide rules.
- Surface enforcement capability gaps honestly in preflight.
- Wire enforcement events into the existing audit taxonomy without inventing a separate format.
- Leave a clean seam for stronger isolation modes in later phases.

## Phase 4 — Team and distribution features (complete)

- ✓ **Signed profile distribution** (4.1)
  - ed25519 signatures on profiles; `sign_profile` command callable from the desktop
  - `verify_signature` on every profile load; `SignatureStatus` surfaced in profile picker
  - `fetch_remote_profile`: HTTPS GET with disk cache fallback at `<store>/.rampart/profile-cache/`
  - Preflight hard-blocks launch when signature status is Invalid

- ✓ **Centralized audit sync with background worker** (4.2)
  - Local audit outbox in persisted state; events queued when sync is configured
  - `sync_audit_events`: drains up to 500 unsent entries per call, POSTs to configured endpoint with Bearer auth
  - `strip_paths` option redacts file path values before sending
  - `configure_sync` / `get_sync_status` Tauri commands; sync settings panel in desktop launcher
  - Background worker: `drain_audit_queue` free function; `trigger_background_sync` spawns thread on every `stop_session`

- ✓ **Org settings** (4.3)
  - `OrgPolicy` + `OrgPolicyScope` structs in policy-core (policy floor + scope: agent type glob + project path glob)
  - `resolve_effective_policy(local, org)` strict merge — most restrictive value wins per dimension
  - `org_policy_applies` scope matching for agent type and project path prefix
  - `preflight_check` annotates diagnostics with `from_org_policy` when org floor is active
  - UI: "Org policy floor active" badge in launcher preflight panel; `[from org policy]` inline tags on affected diagnostics

Reference-driven priorities inside this phase:

- Keep team controls above the local enforcement loop instead of moving core policy decisions into a hosted plane.
- Reuse the local event model and profile schema for sync features instead of inventing separate admin-only formats.
- Treat remote or bridge workflows as access patterns layered on top of local execution, not as a separate product core.

## Phase 5 — Headless, CI, and broader engine support (in progress)

- ✓ **Headless `rampart run` CLI**: `apps/cli/` binary reuses `RampartService` for full enforcement. `rampart run --agent <id> --profile <id> --project <path> [--wsl2]` — preflight to stderr, JSONL events to stdout, clean Ctrl+C shutdown. `rampart list-profiles` enumerates agent-specific presets.
- **WFP violation streaming** (pipeline complete; end-to-end pending): `FwpmNetEventSubscribe0` subscription and `WfpEventMonitor` wiring are in place. The filter installs and subscribes correctly under admin. End-to-end validation against a real agent initiating outbound traffic is not yet confirmed — test instrumentation used a `.cmd` shim while `resolve_app_path` hardcodes `.exe`, causing filter and process image to diverge. Pipeline verified; block-triggers-event not yet exercised in normal use.
- ✓ **Per-agent capability matrices**: `agent_profile_presets()` in policy-core now returns tailored standard+strict profiles for Cursor, Copilot, Goose, OpenCode, and GeminiCli in addition to ClaudeCode/Codex/Aider. Each standard profile allows the agent's known API endpoints; strict profiles deny network entirely. 12 Rust unit tests added.
- AppContainer isolation mode for stronger process-level sandboxing
- Enterprise controls where justified

Reference-driven priorities inside this phase:

- Add headless before enterprise. A CLI path that reuses the daemon's enforcement stack is more valuable earlier than SSO or governance dashboards.
- Add per-agent capability matrices so broader agent support does not weaken the enforcement contract.
- Consider plugin and extension points only after Windows enforcement, explanation, and distribution loops are trustworthy.
- Violation event streaming is the most important UX gap before handing Rampart to external beta users in Windows Native mode.

## What Changed From Research Review

External implementation review does not change Rampart's product position, but it does make several next steps more concrete.

- Real terminal agents need explicit startup orchestration, so Rampart should invest early in agent adapters and preflight checks.
- Permission and mode state become central quickly, so Rampart should mature its profile and audit model before building team administration.
- Session boundaries matter, so launcher, live session, history, and later remote attachment should be modeled as distinct product states.
- Extension seams matter, but plugins, bridge modes, and broader orchestration should remain downstream of the trustworthy local enforcement loop.
