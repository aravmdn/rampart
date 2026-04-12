# Architecture

Rampart is a local-first desktop product for controlling AI coding agents with enforced least privilege.

## Core shape

- Desktop app: project selection, agent selection, profile selection, live session view, and history
- Local daemon: session orchestration, policy resolution, process supervision, persistence, and local APIs
- Policy core: schema validation, profile compilation, templates, and event normalization
- Engine adapters: translate Rampart policy into engine-specific launch and enforcement configuration

## Product loop

1. Choose a project, agent, and profile.
2. Launch a sandboxed session.
3. Capture allows, blocks, and violations.
4. Explain what happened and why.
5. Refine the policy for the next run.

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
