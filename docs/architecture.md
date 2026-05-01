# Architecture

Rampart is a local-first desktop product for controlling AI coding agents with enforced least privilege.

## Core shape

- Desktop app: project selection, agent selection, profile selection, live session view, and history
- Local daemon: session orchestration, policy resolution, process supervision, persistence, and local APIs
- Policy core: schema validation, profile compilation, templates, and event normalization
- Engine adapters: translate Rampart policy into engine-specific launch and enforcement configuration

## Runtime boundary

- `rampartd` depends on an engine trait, not a hard-coded sandbox implementation.
- Engine adapters own binary discovery, capability reporting, and event normalization.
- Current `greywall` adapter is reference integration for non-Windows paths and compatibility testing.
- Windows-first delivery means public docs and capability snapshots must not imply `greywall` is Rampart's Windows runtime.
- The desktop talks to the daemon through a Tauri invoke bridge. Command construction, project detection, profile resolution, local persistence, and history queries stay on the Rust side of that boundary.

## Agent adapter boundary

- Rampart should add an agent adapter layer inside the daemon that is distinct from the enforcement engine adapter layer.
- Agent adapters should own binary discovery, version checks, startup arguments, environment shaping, working-directory rules, and launch diagnostics for each supported tool.
- Enforcement engines should remain focused on sandbox capability reporting, launch translation, and raw event normalization.
- This keeps agent-specific quirks out of the engine contract and makes Windows-first runtime work easier to evolve.

## Local persistence

- Launch selections and session history persist locally through the daemon.
- The current implementation stores launcher selections and session history in a local daemon-managed state file so project, agent, profile, audit, and violation records survive desktop restarts.
- Persisted history is part of the core product loop because blocked actions need to remain visible after a session ends, not only while a session is live.

History should mature into a single session record that includes:

- launch request and resolved launch context
- capability snapshot seen at launch time
- ordered audit and violation events
- stop reason and termination state
- profile identifier or compiled policy version used for the run

## Product loop

1. Choose a project, agent, and profile.
2. Launch a sandboxed session.
3. Capture allows, blocks, and violations.
4. Explain what happened and why.
5. Refine the policy for the next run.

Supporting architecture should therefore include:

- launch preflight checks before session start
- a stable event taxonomy shared across agents and engines
- policy explanations that combine rule match, attempted action, and platform limitation
- a session console surface optimized for live visibility after launch

## Principles

- Local-first before cloud
- Default-deny by default
- Enforcement outside model prompts
- Replaceable engine abstraction
- Honest capability reporting across platforms

## Platform posture

- Windows is the primary product constraint
- macOS and Linux follow after the Windows execution model is solid
- The repo should not assume one engine is the permanent runtime
- Public docs should surface limitations instead of implying unsupported protection

## Windows enforcement engine (Phase 3 — enforcement code complete)

MVP requires real OS-level enforcement. The engine adapter for Windows owns:

- Process containment scoped to the agent and its job tree
- Network allow/block enforcement per profile policy, evaluated before connections complete
- Filesystem write scoping bounded to the project directory
- Audit event emission into the existing `AuditEventKind` taxonomy

The engine adapter boundary keeps enforcement implementation details out of the daemon and desktop layers. Stronger isolation modes are preserved as a clean seam for later phases.

### Phase 3 implementation status

**Process containment** — complete. `WindowsJob` creates a Job Object with `KILL_ON_JOB_CLOSE` and assigns the agent process at spawn. The OS kills the entire process tree when the job handle is dropped (session end or Rampart exit). Implemented in `engine-windows`, wired in `LocalProcessRunner`.

**Filesystem write restriction** — complete. `set_process_low_integrity()` sets the agent's process token to Low Integrity (S-1-16-4096) post-spawn. The OS denies writes to all Medium-or-higher integrity paths without custom hooks. `patch_project_low_integrity_label()` sets a Low mandatory SACL label on the project root so the agent can write to its own working directory while remaining blocked everywhere else.

**Network enforcement** — complete. `WfpNetworkGuard` opens a dynamic WFP engine session and installs per-application-ID outbound BLOCK filters on both IPv4 and IPv6 ALE connect layers. The NT device path is resolved from the Win32 executable path via `QueryDosDeviceW`. Filters auto-remove when the session handle is dropped.

**Audit trail** — complete. `EtwAuditProvider` registers a Rampart ETW provider and emits every session lifecycle and audit event via `EventWriteString`. Capturable with standard Windows tracing tools. Wired into `RampartDaemon` for launch, stop, and all ingested events.

**WSL2 isolation mode** — complete. Users with WSL2 installed can select a stronger isolation mode that runs the agent inside a Linux VM. `detect_wsl2()` probes at preflight. `Wsl2Enforcer` reports `Supported` capabilities. `LocalProcessRunner` wraps the command as `wsl --cd <linux_path> -- <command>` and skips Win32 enforcement hooks that have no effect inside the VM.

**Runtime validation** — complete (2026-04-28). Tests 1–5 all pass: Job Object containment, Low Integrity token, project root SACL, WFP outbound block, ETW audit capture. One known limitation: OS-level blocks do not stream back as violation events in Windows Native mode. Blocks fire correctly; UI feedback is silent. WSL2 mode does surface events.

## Phase 1 and early Phase 2 implementation status

The following Phase 1 priorities are complete:

- Agent-specific launch adapters: `AgentAdapter` in rampartd owns command, default args, env, and terminal-first flag per agent tool. `LocalProcessRunner` uses adapters rather than bare command name lookup.
- Launch preflight diagnostics: `run_preflight()` checks project directory, agent binary on PATH, capability gaps, and policy/capability compatibility before launch. Exposed via `preflight_check` Tauri command.
- Launcher and session console split: the desktop shell has three distinct views — launcher, session console, and history. The launcher handles pickers, preflight, capability warnings, and recent history. The session console handles live status, audit stream, and violation explanations. The history view provides full per-session inspection.
- Capability warnings before launch: flattened capability items are computed server-side and surfaced in the launcher. An explicit warning panel appears when unsupported capabilities are detected. Launch is disabled until preflight passes.
- Terminal-first handoff: agents flagged as `terminal_first` show a handoff note in the launcher. Rampart controls enforcement and visibility; the agent keeps its native terminal interaction.

The following early Phase 2 priorities are also complete:

- Full session records: `SessionHistoryRecord` carries an optional capability snapshot captured at launch time. Each history entry is self-contained with launch context, events, and violations.
- Violation explanations: `ViolationExplanation` in policy-core separates the policy reason, platform/engine limitation, and remediation hint into structured fields. `ViolationEvent` carries an optional explanation.
- Stable audit taxonomy: `AuditEventCategory` (SessionLifecycle / PolicyEnforcement / SystemAlert) and domain-specific `AuditEventKind` variants (FilesystemAllowed, FilesystemBlocked, NetworkAllowed, NetworkBlocked, ProcessAllowed, ProcessBlocked) replace the generic pair for new events.
- History detail view: the desktop shell has a dedicated history view. `HistoryList` shows session metadata; `HistoryDetailPanel` shows the full audit trail and violations for a selected session. The launcher's Recent History section links to it.
- Agent-aware profile presets: `agent_profile_presets()` in policy-core returns tailored standard and strict presets for ClaudeCode, Codex, and Aider. `launch_context()` filters to show only the selected agent's presets. `preflight_check()` resolves profiles from the agent-specific list first.

## Phase 4 implementation status

**Signed profile distribution** — complete. `ProfileSignature` and `SignatureStatus` in policy-core. `sign_profile` / `verify_signature` via `ed25519-dalek`. Signature field on `Profile`; `SignatureStatus` surfaced in profile picker. `fetch_remote_profile` performs HTTPS GET with disk cache fallback. Preflight hard-blocks launch when status is Invalid.

**Centralized audit sync** — complete. `SyncConfig` persisted in local state. `AuditQueueEntry` enqueued in `persist_history_record` when sync is configured. `sync_audit_events` batches up to 500 unsent entries and POSTs to the endpoint with Bearer auth. Optional `strip_paths` redaction. `configure_sync` / `get_sync_status` Tauri commands; sync settings panel in desktop launcher. Background worker (`drain_audit_queue` + `trigger_background_sync`) drains the outbox on every `stop_session`.

**Org settings** — complete. `OrgPolicy` + `OrgPolicyScope` in policy-core. `resolve_effective_policy(local, org)` takes the most restrictive value per dimension; empty org lists are treated as "no opinion" to avoid locking down dimensions the org didn't specify. `org_policy_applies` matches agent type glob and project path prefix. `configure_org_policy_url` / `fetch_org_policy` / `current_org_policy` Tauri commands. Preflight annotates diagnostics with `from_org_policy` when the floor is active; the launcher shows an "Org policy floor active" badge.

## Phase 5 implementation status (in progress)

**Headless CLI** — complete. `apps/cli/` provides a `rampart` binary. `rampart run --agent <id> --profile <id> --project <path> [--wsl2]` runs preflight to stderr and streams audit + violation events as JSONL to stdout. All Windows enforcement primitives apply through the same `RampartService` as the desktop. `rampart list-profiles --agent <id> --project <path>` lists agent-specific presets.

**WFP violation streaming** — complete (privilege validation pending runtime test). `WfpEventMonitor` subscribes via `FwpmNetEventSubscribe0` and surfaces blocked connections as `SessionEventRecord::Audit` entries in the active session. Events are drained and persisted at `stop_session`. If the subscription fails at Medium integrity (access-denied), a descriptive hint is emitted to stderr. Full violation streaming in Windows Native mode without elevation remains an open UX gap.

## Research-informed lessons

External implementation research is useful to Rampart in a narrow, practical way:

- startup orchestration: do preflight work before opening an active session
- command and mode partitioning: keep launcher, session, and later bridge states explicit
- permission modeling: treat allow, deny, ask, bypass, and plan-style modes as durable domain vocabulary
- session lifecycle: persist enough state for history and later resume or attachment workflows
- extension pressure: preserve seams for plugins, skills, and remote control without prioritizing them ahead of the local enforcement loop

The useful takeaway is not to rebuild any one existing agent product. The useful takeaway is to shape Rampart so real agent complexity has a place to land later without forcing a rewrite of the launcher, daemon contracts, or policy model.
