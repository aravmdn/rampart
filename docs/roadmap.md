# Rampart roadmap

This roadmap reflects the current product reference document while keeping the public docs focused on product and architecture rather than internal operating plans.

## Phase 0: Foundation

- Finalize repository structure and module boundaries
- Document architecture, threat model, and platform constraints
- Define policy model and event model
- Establish the local-first product language and enforcement boundaries

## Phase 1: Local desktop MVP

- Tauri desktop shell
- macOS and Linux support through the initial `greywall` adapter
- Project picker, agent picker, and profile templates
- One-click sandboxed session launch
- Live session state and violation stream

Outcome:

- a single developer can launch a supported agent inside a constrained local session and understand blocked behavior in real time

## Phase 2: Policy UX and local history

- Project auto-detection and profile suggestions
- Safer profile editing without raw engine syntax
- Persistent local session history
- Clearer violation explanations and diagnostics

Outcome:

- Rampart becomes a usable daily workflow tool instead of only a launch wrapper

## Phase 3: Team-ready local controls

- Export and import of repository-backed policy files
- Local alerts and suspicious-activity thresholds
- Kill switch workflows
- Team-oriented audit and policy sharing boundaries

Outcome:

- small teams can standardize local agent controls without requiring a cloud-first product

## Phase 4: Headless and CI surfaces

- Linux headless mode
- CLI-based session execution
- CI integration patterns such as GitHub Actions
- Structured session summary output

Outcome:

- Rampart works in both developer desktops and automation environments

## Phase 5: Enterprise-enabling features

- Team dashboard and audit aggregation where justified
- SSO and data residency options
- Compliance-oriented operational controls

Outcome:

- the product can support paid team and enterprise deployments without compromising the local-first free tier

## Platform note

- macOS and Linux are the current v1 platforms
- Windows remains a future platform that will require a separate enforcement approach
- public roadmap language must stay explicit about capability differences across OSes
