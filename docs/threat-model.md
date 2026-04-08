# Rampart threat model

## Goal

Rampart exists to reduce the blast radius of AI coding agents running on developer machines by enforcing least privilege at the execution layer and exposing the resulting behavior to the user.

This threat model is intentionally scoped to the local product and its immediate team extensions.

## In scope

- AI coding agents reading sensitive local files
- AI coding agents writing outside intended project boundaries
- AI coding agents making unexpected outbound network requests
- unclear or missing evidence of what an agent attempted during a session
- users overtrusting protections that the current platform cannot fully enforce

## Out of scope

- prompt or completion content filtering
- malware analysis beyond the agent sandboxing use case
- Docker-native container isolation for agent workloads
- Windows-specific enforcement details before a Windows runtime exists

## Protected assets

- source code and repository contents
- `.env` files and local secrets
- SSH keys and other developer credentials
- project-scoped configuration
- local audit history and policy definitions

## Actors

- benign but overly-permissive AI coding agents
- misconfigured agents or tools
- prompt-injected or compromised agent workflows
- users who misunderstand the product's current platform guarantees

## Trust boundaries

- desktop application
- local daemon
- policy compilation layer
- enforcement adapter
- underlying sandbox engine
- child AI agent process
- local filesystem
- outbound network destinations
- optional team sync boundary in later phases

## Core assumptions

- the enforcement engine is the source of truth for what was technically blocked or allowed
- product policy is only meaningful if it maps to verified platform capabilities
- local logs are sensitive operational records and should stay local by default
- macOS and Linux do not necessarily provide equivalent enforcement or observability

## Main threat scenarios

## 1. Sensitive file read outside project scope

Example:

- an agent attempts to read `~/.ssh`, shell history, or secret files while working in a project

Desired behavior:

- read is blocked by the enforcement layer
- violation is captured and explained clearly

## 2. Write outside allowed roots

Example:

- an agent writes to another repository, a user config directory, or a credential store

Desired behavior:

- write is denied unless explicitly allowed
- the session record shows the attempted path and policy reason

## 3. Unexpected network access

Example:

- an agent reaches outbound domains unrelated to the task or policy

Desired behavior:

- outbound activity is blocked or surfaced according to platform capability
- UI and docs explicitly describe where network control is limited

## 4. Misleading safety signals

Example:

- the product UI implies complete network or filesystem protection on a platform where only partial coverage exists

Desired behavior:

- capability gaps are shown before and during the session
- unsupported policy rules are rejected or downgraded visibly

## 5. Audit loss or insufficient forensic value

Example:

- a session ends and the user cannot determine what was attempted, blocked, or terminated manually

Desired behavior:

- Rampart persists local session summaries, violations, and termination events with enough context to answer what happened

## Known technical constraints

## Temporary files and rename semantics

- some edit flows rely on temporary paths and later renames
- filesystem policy must account for the gap between user intent and syscall-level enforcement

## Platform capability mismatch

- macOS and Linux features are not guaranteed to match
- network monitoring and proxy behavior may differ materially

## Upstream engine dependency

- the product currently depends on `greywall` behavior and version compatibility for the initial macOS/Linux path

## Security requirements

- default-deny policy posture
- policy authoring separated from enforcement and observability
- user-visible explanation for blocked behavior
- explicit capability reporting by platform and engine
- no cloud dependency required for the free tier

## Verification expectations

Before claiming protection in a feature or release, verify:

- the session can be launched under policy
- blocked actions are actually blocked
- the event is captured and normalized
- the user sees a clear explanation of what happened

At least one negative-path test or manual repro should exist for blocked access scenarios.
