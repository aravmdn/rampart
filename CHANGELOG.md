# Changelog

All notable changes to this project will be documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-05-07

### Added

**Agent adapters and launch orchestration**
- `AgentAdapter` per-agent struct in rampartd: command, default args, env shaping, terminal-first flag, working-directory rules
- `run_preflight()` and `PreflightReport`: project directory check, agent binary on PATH, capability gap detection, policy/capability compatibility check; exposed via `preflight_check` Tauri command
- Agent-aware profile presets (`agent_profile_presets()`): standard and strict profiles for ClaudeCode, Codex, Aider, Cursor, Copilot, Goose, OpenCode, and GeminiCli (8 agents, 16 presets, 12 unit tests)
- `profiles_for_agent()` and agent-filtered `launch_context()`: profile picker shows only presets relevant to the selected agent

**Session console and live visibility**
- Launcher, session console, history, and profile editor as four distinct views in the desktop shell
- Live audit stream polling (3-second interval) in session console
- Capability Snapshot panel in launcher: shows enforcement state per dimension before launch
- Capability Warnings panel: blocks launch when unsupported capabilities are detected
- Terminal-first handoff note for terminal-oriented agents (Claude Code, Codex, Aider)

**Session history**
- `SessionHistoryRecord`: self-contained session record with launch context, capability snapshot, ordered audit events, violations, stop reason, and profile version
- History view: `HistoryList` + `HistoryDetailPanel` in shared-ui; full per-session audit trail and violation inspection without leaving the desktop app
- Recent History panel in launcher: links to the three most recent sessions

**Violation explanations and policy refinement**
- `ViolationExplanation` in policy-core: structured fields for policy reason, platform/engine limitation, and remediation hint
- `AuditEventCategory` (SessionLifecycle / PolicyEnforcement / SystemAlert) and domain-specific `AuditEventKind` variants (FilesystemAllowed, FilesystemBlocked, NetworkAllowed, NetworkBlocked, ProcessAllowed, ProcessBlocked)
- `ProfileEditorPanel` in shared-ui: filesystem paths, network hosts, allowed commands, and default-action toggles in product language (no raw engine syntax)
- Safe policy refinement flow: "Adjust policy" on any violation opens the profile editor pre-populated with a targeted rule suggestion derived from violation type and blocked target

**Windows enforcement engine (runtime-validated)**
- `WindowsJob`: Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`; entire agent process tree terminated on session end or Rampart exit
- `set_process_low_integrity()`: agent process token set to Low Integrity (S-1-16-4096) post-spawn; OS denies writes to all Medium-or-higher integrity paths without custom hooks
- `patch_project_low_integrity_label()`: Low mandatory SACL on project root so agent can write to its own working directory
- `WfpNetworkGuard`: per-application-ID outbound BLOCK filters on IPv4 and IPv6 ALE connect layers via dynamic WFP session; auto-removed on session end; no kernel driver required
- `EtwAuditProvider`: Rampart ETW provider (GUID 7E5A6B4C-...) emitting all session lifecycle and audit events via `EventWriteString`; capturable with standard Windows tracing tools
- `IsolationMode` enum (WindowsNative / Wsl2) on Session and LaunchSessionRequest
- `Wsl2Enforcer`: agent runs inside Linux VM via `wsl --cd <linux_path> -- <command>`; Win32 enforcement hooks skipped inside VM
- `detect_wsl2()`: preflight probe; WSL2 isolation surfaced as opt-in stronger isolation when detected
- `win_path_to_wsl()`: converts `C:\...` to `/mnt/c/...` for WSL working directory

**Team and distribution features**
- Signed profile distribution: ed25519 signatures (`sign_profile` / `verify_signature`); `SignatureStatus` surfaced in profile picker; `fetch_remote_profile` HTTPS GET with disk cache fallback; preflight hard-blocks launch on Invalid signature
- Centralized audit sync: local `AuditQueueEntry` outbox; `sync_audit_events` drains up to 500 entries per call, POSTs to configured endpoint with Bearer auth; optional `strip_paths` redaction; `configure_sync` / `get_sync_status` Tauri commands; sync settings panel in launcher
- Background sync worker: `trigger_background_sync` spawns thread on every `stop_session`; drains outbox automatically
- Org settings: `OrgPolicy` + `OrgPolicyScope` structs; `resolve_effective_policy` strict merge (most restrictive per dimension); `from_org_policy` annotation on preflight diagnostics; "Org policy floor active" badge and inline `[from org policy]` tags in launcher

**Headless CLI**
- `apps/cli/` binary (`rampart`): `run` and `list-profiles` subcommands
- `rampart run --agent <id> --profile <id> --project <path> [--wsl2]`: preflight to stderr, JSONL event stream to stdout, clean Ctrl+C shutdown
- Full enforcement primitives applied through the same `RampartService` as the desktop
- WFP violation streaming: `WfpEventMonitor` subscribes via `FwpmNetEventSubscribe0`; blocked connections surface as session events; pipeline verified under admin

**Documentation**
- `docs/threat-model.md`: honest accidental-overreach framing; enforcement primitives with bypass surface documented; known gaps listed
- `docs/architecture.md`: module boundaries, IPC layer, engine adapter contract, Phase 3–5 implementation status
- `docs/roadmap.md`: phase-by-phase status with implementation notes
- `GETTING_STARTED.md`: first-time user guide covering prerequisites, launch, session walkthrough, CLI, and troubleshooting

### Changed

- Profile picker shows only presets relevant to the selected agent (was: all presets regardless of agent)
- Preflight diagnostics annotated with `[from org policy]` when org policy floor is active
- `launch_context()` returns agent-filtered profile list

### Fixed

- `WindowsJob`: initial implementation used `BasicLimitInformation` (class 2); `KILL_ON_JOB_CLOSE` requires `ExtendedLimitInformation` (class 9) on Windows 11 — fixed and runtime-verified
- `WfpFilterAdd0`: returned `FWP_E_NULL_DISPLAY_NAME` due to null `filter.displayData.name` — fixed and runtime-verified
- Windows agent spawn: `Command::new("claude")` failed for npm `.cmd` shims because `CreateProcessW` does not resolve PATHEXT extensions; added `cmd /C` dispatch for `.cmd` and `.bat` shims and `CREATE_NEW_CONSOLE` for terminal-first agents
- `resolve_app_path`: now tries `.exe`, `.cmd`, `.bat` in PATHEXT order so WFP filters target the actual agent binary even for npm/pnpm-installed shims
- WFP block events now produce both an audit row (Audit Stream) and a violation row (Violation View + Adjust policy flow) — previously only the audit row, so the GUI Violation View stayed empty when WFP fired
- Profile picker `[signed]` badge: built-in presets are now signed at emission with `BUILTIN_SIGNING_SEED`; `verify_signature` returns `Valid` for all first-party presets
- Failed launch: when the daemon returns `status=failed`, the desktop now reads back the last audit message and surfaces it as a "Launch error" panel instead of a frozen status badge
- Multi-project support: project picker is no longer hardcoded to the current git repo; `add_project` / `remove_project` Tauri commands and an Add Project form in the launcher allow any local directory to be registered
- Agent picker shows all 8 supported agents (was previously truncated to 2)
- Sync status loaded on app mount instead of only after the first Save click; Sync Now button correctly enabled when the daemon already has a persisted config
- Profile list re-fetches when the agent selection changes so the picker shows per-agent presets without a manual reload
- Preflight panel renders an empty-state placeholder when no diagnostics apply
- Org policy fetch / sync configure errors render under their panels instead of throwing unhandled rejections

### Security

- All enforcement primitives operate outside model prompts; policy is deterministic and OS-enforced
- Default-deny baseline: agents must have explicit allow rules for filesystem paths, network hosts, and allowed processes
- Job Object breakaway disallowed by default (`JOB_OBJECT_LIMIT_BREAKAWAY_OK` not set)
- Low Integrity token scopes filesystem writes to project root only; system directories and user profile are blocked without custom hooks
- WFP filters are per-application-ID and scoped to the session; no machine-wide rules; filters auto-remove on session end
- Signed profile distribution: ed25519 signatures; Invalid signature hard-blocks launch
- ETW audit trail: every session lifecycle and enforcement event captured by OS-level tracing

[0.1.0]: https://github.com/rampart-dev/rampart/releases/tag/v0.1.0
