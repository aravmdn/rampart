# Roadmap

Rampart ships in phases that lock down the local enforcement loop first before expanding to team and CI features.

---

## Phase 1 — Desktop shell and launch flow (complete)

- Desktop shell: project picker, agent picker, profile picker, live session visibility
- Agent-specific launch adapters instead of treating every agent as a bare executable
- Launch preflight diagnostics: explains startup failures before a session begins
- Launcher and session console as separate product states
- Capability warnings before launch; launch disabled until preflight passes
- Terminal-first handoff preserved: Rampart controls enforcement while the agent keeps its native terminal

## Phase 2 — Profile editing, history, and policy refinement (complete)

- Full session records: launch context, capability snapshot, audit trail, and violations persisted together
- Rule-linked violation explanations in product language, including platform limitation notes
- Agent-aware profile presets: standard and strict profiles for ClaudeCode, Codex, and Aider
- Stable audit event taxonomy: `AuditEventCategory` and `AuditEventKind` variants across engines and agents
- Profile editor: filesystem paths, network hosts, allowed commands; no raw engine syntax
- Safe policy refinement: "Adjust policy" on a violation opens the editor pre-populated with a targeted rule suggestion
- History view: per-session audit trail and violation inspection in the desktop app

## Phase 3 — Windows enforcement engine (complete, runtime-validated)

MVP gate closed. All enforcement mechanisms verified on Windows 11 under admin (2026-04-28, re-confirmed 2026-05-06).

- **Process containment**: `WindowsJob` creates a Job Object with `KILL_ON_JOB_CLOSE`. The OS kills the entire agent process tree on session end or Rampart exit. Breakaway disallowed by default.
  - Fix applied during validation: `KILL_ON_JOB_CLOSE` requires `ExtendedLimitInformation` (class 9), not `BasicLimitInformation` (class 2), on Windows 11.
- **Filesystem write scoping**: `set_process_low_integrity()` sets the agent token to Low Integrity (S-1-16-4096). `patch_project_low_integrity_label()` sets a Low SACL on the project root so the agent can write to its working directory.
- **Network enforcement**: `WfpNetworkGuard` installs per-app-ID outbound BLOCK filters on IPv4 + IPv6 ALE connect layers via a dynamic WFP session. Filters auto-remove on session end. No kernel driver required.
  - Fix applied during validation: `FwpmFilterAdd0` returned `FWP_E_NULL_DISPLAY_NAME` due to a null `filter.displayData.name`; fixed.
- **ETW audit trail**: `EtwAuditProvider` emits all session lifecycle and audit events. Capturable with standard Windows tracing tools.
- **WSL2 isolation mode**: `detect_wsl2()` at preflight; `Wsl2Enforcer`; agent wrapped as `wsl --cd <linux_path> -- <command>`; Win32 enforcement hooks skipped inside the VM.

## Phase 4 — Team and distribution features (complete)

- **Signed profile distribution**: ed25519 signatures on profiles; `SignatureStatus` surfaced in profile picker; HTTPS fetch with disk cache fallback; preflight hard-blocks on Invalid signature
- **Centralized audit sync**: local `AuditQueueEntry` outbox; `sync_audit_events` drains up to 500 entries per call, POSTs with Bearer auth; optional `strip_paths` redaction; background worker drains on every `stop_session`
- **Org settings**: `OrgPolicy` + `OrgPolicyScope`; `resolve_effective_policy` strict merge; `from_org_policy` annotation on preflight diagnostics; "Org policy floor active" badge in launcher

## Phase 5 — Headless, CI, and broader engine support (complete except where noted)

- **Headless CLI** (complete): `apps/cli/` binary. `rampart run --agent <id> --profile <id> --project <path> [--wsl2]` — preflight to stderr, JSONL events to stdout, clean Ctrl+C shutdown. All Windows enforcement primitives applied through `RampartService`.
- **Per-agent capability matrices** (complete): standard + strict profiles for Cursor, Copilot, Goose, OpenCode, and GeminiCli added to `agent_profile_presets()`; 12 unit tests
- **WFP violation streaming** (pipeline complete; end-to-end pending): `WfpEventMonitor` subscribes via `FwpmNetEventSubscribe0`; filter installs and subscribes correctly under admin. End-to-end validation against a real agent initiating outbound traffic is not yet confirmed — the test instrumentation used a `.cmd` shim while `resolve_app_path` hardcodes `.exe`, causing filter and process image to diverge. Blocks fire at OS level; console surfacing of those blocks is not yet exercised in normal use.
- **AppContainer isolation mode** (deferred): the engine adapter architecture preserves a clean seam. AppContainer provides a separate security principal per session and is the most principled long-term isolation primitive. Deferred to a future phase.

---

## What's Next

### Installer and packaging

No installer exists yet. Users must build from source. The next distribution step is a signed MSIX or NSIS installer that:

- bundles the Tauri desktop app and the `rampart` CLI binary
- handles the Administrator privilege requirement at install time
- provides a Start Menu entry and optional PATH registration for the CLI

### AppContainer isolation mode

Windows AppContainer is the highest-fidelity isolation primitive available without a hypervisor. It assigns each session a unique security principal, enabling per-app filesystem and network ACLs at the OS layer with no cleanup burden on session end. The engine adapter seam is ready; implementation is the next enforcement investment after packaging.

### End-to-end WFP violation streaming

The WFP block-to-console pipeline is wired and verified under admin. The remaining work is confirming that a blocked outbound connection by a real agent (not a `.cmd` shim) surfaces as a violation event in the session console. This closes the only known gap in the Windows Native enforcement UX.

### macOS and Linux

`greywall` is a reference adapter for non-Windows paths. macOS and Linux enforcement is not a current priority; the Windows execution model must be solid and shipped first.
