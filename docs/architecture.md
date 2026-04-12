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

## Reference-informed lessons

The Claude Code reference folder is useful to Rampart in a narrow, practical way:

- startup orchestration: do preflight work before opening an active session
- command and mode partitioning: keep launcher, session, and later bridge states explicit
- permission modeling: treat allow, deny, ask, bypass, and plan-style modes as durable domain vocabulary
- session lifecycle: persist enough state for history and later resume or attachment workflows
- extension pressure: preserve seams for plugins, skills, and remote control without prioritizing them ahead of the local enforcement loop

The useful takeaway is not to rebuild Claude Code. The useful takeaway is to shape Rampart so real agent complexity has a place to land later without forcing a rewrite of the launcher, daemon contracts, or policy model.
