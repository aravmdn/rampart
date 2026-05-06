# Threat Model

Rampart exists because AI coding agents can inherit broad user permissions and touch
data they should not reach.

## Assets

- Source code
- Local secrets and credentials
- SSH keys and shell history
- Network access
- Session history and audit data

## Main threats

- An agent reads files outside the intended project scope
- An agent writes to sensitive paths or exfiltrates local data
- An agent makes outbound network calls that violate policy
- A buggy or compromised agent attempts process abuse or persistence
- A user mistakes a prompt instruction for a real security boundary

## Security stance

- Enforcement must happen outside the model prompt
- Policies should be deterministic and explicit
- Default-deny is the baseline
- Capability limits must be visible to the user before launch
- Logs should explain blocked behavior without exposing private secrets

## Product boundary

Rampart is a policy and visibility layer around agent execution. It is not a generic
observability platform and not a prompt firewall.

---

## What Rampart is designed to contain

Rampart targets **accidental overreach by well-behaved AI coding agents** — agents that
follow the tool call API, operate through the normal Win32 process model, and do not
attempt to bypass security controls.

The prototypical failure modes Rampart prevents:

- An agent writes files outside the project root (e.g., edits shell config or SSH keys
  as a side effect of a task).
- An agent spawns a subprocess that outlives the session and continues running after the
  user closes Rampart.
- An agent makes outbound network calls to unexpected hosts (e.g., exfiltrates context
  to an unintended endpoint).

These are the actual risks posed by today's AI coding agents. Rampart addresses them
with OS-level primitives so that the constraints are enforced regardless of which tool
call the agent uses internally.

---

## What Rampart does NOT contain

Rampart is **not** an adversarial sandbox. It does not protect against:

- A process that issues direct NTDLL system calls (bypassing Win32 API hooks).
- A process that uses Windows AppContainer escape techniques or exploits kernel
  vulnerabilities.
- An agent that is deliberately malicious and aware of its containment environment.
- Side-channel attacks, memory-based exploitation, or privilege escalation.

If your threat model includes adversarial code, you need a hypervisor boundary (WSL2
with Hyper-V, Windows Sandbox, or a VM). Rampart's WSL2 mode (Phase 3 fast-follow)
provides stronger isolation for users who already have Hyper-V available.

---

## Current enforcement primitives (Phase 3 MVP)

### Windows Job Objects — process tree containment

**What it does:**
When a session launches, Rampart creates a Windows Job Object with
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and assigns the agent process to it. Every child
process the agent spawns inherits job membership automatically (Windows 8+ nested job
semantics). When Rampart drops the job handle — at session end, on stop, or on crash —
the OS kills every process still in the job.

**What it guarantees:**
No process from the agent session survives after Rampart closes the session. The agent
cannot escape the job by spawning a detached subprocess.

**What it does not guarantee:**
Job Objects do not restrict file system access or network access. A process inside the
job can write arbitrary files and make arbitrary network calls until the ACL and WFP
layers are in place.

**Bypass surface:**
A process can call `CreateProcess` with `CREATE_BREAKAWAY_FROM_JOB` only if the job
explicitly allows it. Rampart does not set `JOB_OBJECT_LIMIT_BREAKAWAY_OK`, so breakaway
is disallowed by default. Well-behaved agents will not attempt this.

---

### Filesystem write containment — Low Integrity token + project root SACL

**What it does:**
At session start, Rampart sets the agent's process token to Low Integrity (S-1-16-4096)
via `set_process_low_integrity()`. The OS denies writes to all Medium-or-higher integrity
paths — user profile, system directories, temp — without custom hooks. `patch_project_low_integrity_label()`
sets a Low mandatory label on the project root so the agent can write to its own working
directory while remaining blocked everywhere else.

**Current status:** Implemented and runtime-verified. Reported as `Supported` in the
capability snapshot.

---

### Network enforcement — Windows Filtering Platform

**What it does:**
`WfpNetworkGuard` opens a dynamic WFP engine session and installs per-application-ID
outbound BLOCK filters on both IPv4 and IPv6 ALE connect layers. The NT device path is
resolved from the Win32 executable path via `QueryDosDeviceW`. Filters auto-remove when
the session handle is dropped. No kernel driver is required.

**Current status:** Implemented and runtime-verified under admin. Reported as `Supported`
in the capability snapshot.

**Known edge case:** `resolve_app_path` resolves agent commands via `SearchPathW` with a
hardcoded `.exe` extension. Agent shims distributed as `.cmd` or `.bat` would result in
the WFP filter being applied to the wrong image path. This is not a defense gap against
well-behaved agents (which are launched as `.exe` processes) but is worth noting for
shim-style deployments.

**Violation streaming:** The `WfpEventMonitor` subscription pipeline is wired and verified
under admin. End-to-end validation (a blocked outbound connection surfaces as a JSONL event
in the session console) is not yet confirmed: test instrumentation used a `.cmd` shim,
causing the filter and spawned process image to diverge. Pipeline complete; end-to-end
pending.

---

### ETW audit trail

**What it does:**
`EtwAuditProvider` registers a Rampart ETW provider (GUID 7E5A6B4C-...) and emits every
session lifecycle and audit event via `EventWriteString`. Events are capturable with
standard Windows tracing tools. ETW is audit-only — it does not block operations.

**Current status:** Implemented and runtime-verified.

---

## Capability snapshot honesty

Rampart's preflight check and launcher UI reflect actual enforcement state, not an
aspirational one. The current capability snapshot reports:

| Capability             | Status      |
|------------------------|-------------|
| Process enforcement    | Supported   |
| Process termination    | Supported   |
| Filesystem enforcement | Supported   |
| Network enforcement    | Supported   |

Users see this in the launcher before every session. WSL2 mode is surfaced as an opt-in
stronger isolation option when WSL2 is detected at preflight.

**Known cosmetic gap:** In WSL2 isolation mode, `detect_capabilities` still returns the
Windows-native capability snapshot rather than the WSL2 Linux capabilities. Preflight
in WSL2 mode therefore surfaces Windows-native capability warnings. This is a UI artifact
with no enforcement consequence.

---

## WSL2 stronger isolation mode (Phase 3, complete)

For users with Hyper-V available, running the agent inside WSL2 provides a Hyper-V
boundary, which is meaningfully stronger than Win32 Job Objects alone. `detect_wsl2()`
probes at preflight. `Wsl2Enforcer` reports `Supported` capabilities. `LocalProcessRunner`
wraps the command as `wsl --cd <linux_path> -- <command>` and skips Win32 enforcement
hooks that have no effect inside the VM.

WSL2 mode is surfaced as an opt-in stronger isolation option in the launcher when WSL2
is detected. Runtime-verified (2026-05-06).

---

## Known gaps and edge cases

The following are documented edge cases with no current mitigation plan. They do not
change the accidental-overreach threat model framing but are worth knowing.

- **`resolve_app_path` hardcodes `.exe`**: agent commands are resolved via `SearchPathW`
  with a hardcoded `.exe` extension. Agent shims distributed as `.cmd` or `.bat` files
  would have WFP filters applied to the wrong image path, leaving network enforcement
  ineffective for that agent. This is not a defense gap against well-behaved agents
  (all first-class supported agents are launched as `.exe` processes) but shim-style
  deployments should be aware of this.

- **WSL2 capability snapshot reuses Windows-native preflight**: `detect_capabilities`
  returns the Windows-native snapshot regardless of isolation mode. Preflight in WSL2
  mode therefore surfaces Windows-native capability warnings rather than WSL2-specific
  ones. Cosmetic only; enforcement is not affected.

---

## Phase 5 roadmap: AppContainer

Windows AppContainer is the most principled long-term isolation primitive (used by
Edge, Windows Sandbox, and UWP). It provides a separate security principal per session,
enabling per-app network and filesystem rules at the OS ACL layer with no cleanup
burden. The engine adapter architecture preserves a clean seam for AppContainer
integration in Phase 5.
