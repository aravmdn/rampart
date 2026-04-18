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

### Filesystem write containment — directory ACLs (planned, not yet applied)

**Intended behavior:**
At session start, Rampart will add a temporary DACL deny-write entry on directories
outside the project root for the agent process SID. At session end, the entry is
removed.

**Current status:** Not yet implemented. Filesystem enforcement is reported as
`Unsupported` in the capability snapshot. Policy filesystem rules are stored in the
profile but not enforced at the OS level.

---

### Network enforcement — Windows Filtering Platform (planned, not yet applied)

**Intended behavior:**
Rampart will add a per-PID WFP filter that blocks outbound connections to hosts not in
the profile's `allowed_hosts` list. WFP rules are applied at session start and removed
at session end. No kernel driver is required for userspace WFP callouts.

**Current status:** Not yet implemented. Network enforcement is reported as
`Unsupported` in the capability snapshot. Policy network rules are stored in the profile
but not enforced.

---

### ETW audit trail (planned, not yet wired)

**Intended behavior:**
ETW providers for file I/O (`Microsoft-Windows-Kernel-File`) and network
(`Microsoft-Windows-Kernel-Network`) will feed events into the existing `AuditEventKind`
taxonomy. ETW is **audit only** — it does not block operations. Blocking is handled by
ACLs and WFP before the operation completes.

**Current status:** Not yet wired. The `AuditEventKind` taxonomy is defined and ready;
ETW ingestion is the missing piece.

---

## Capability snapshot honesty

Rampart's preflight check and launcher UI reflect actual enforcement state, not an
aspirational one. Until ACLs and WFP are applied, the capability snapshot reports:

| Capability             | Status      |
|------------------------|-------------|
| Process enforcement    | Supported   |
| Process termination    | Supported   |
| Filesystem enforcement | Unsupported |
| Network enforcement    | Unsupported |

Users see this in the launcher before every session. Profiles with filesystem or network
rules generate a "policy compatibility" warning in the preflight report explaining that
the rules are present but not currently enforced.

---

## Fast-follow: WSL2 stronger isolation mode

For users with Hyper-V available, running the agent inside WSL2 provides Linux-native
enforcement (Landlock filesystem isolation + seccomp syscall filtering) via the existing
`engine-greywall` adapter. This is a Hyper-V-boundary isolation, which is meaningfully
stronger than Win32 Job Objects alone.

Rampart will detect WSL2 availability at preflight and surface "Run in WSL2 (stronger
isolation)" as an opt-in mode in the launcher.

---

## Phase 5 roadmap: AppContainer

Windows AppContainer is the most principled long-term isolation primitive (used by
Edge, Windows Sandbox, and UWP). It provides a separate security principal per session,
enabling per-app network and filesystem rules at the OS ACL layer with no cleanup
burden. The engine adapter architecture preserves a clean seam for AppContainer
integration in Phase 5.
