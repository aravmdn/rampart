# Product Reference Document

## Product

Rampart is a Windows-first, local-first security product for AI coding agents. It launches agent sessions through Rampart-owned controls, applies profile-driven policy, records what happened, and explains blocked actions in product language.

## Core user loop

1. Choose project.
2. Choose installed agent.
3. Choose or accept profile.
4. Launch session through Rampart.
5. Watch live session state, violations, and explanations.
6. Stop session and review local history.

## V1 boundaries

- Desktop app is launch-and-control shell, not replacement chat client.
- Enforcement lives outside prompts and outside UI.
- Local daemon owns orchestration, supervision, policy compilation, and persistence.
- Engine layer stays replaceable. `greywall` is reference adapter, not claimed Windows runtime.
- Free local product does not require cloud dependency.

## Success criteria

- Developers can run supported agents through Rampart with default-deny profiles.
- Rampart surfaces capability limits before launch.
- Rampart records launch, allow, block, stop, and policy-related events locally.
- Violations are understandable without reading raw engine output.

## Non-goals

- Generic AI governance platform
- Mandatory cloud control plane for local use
- Enterprise admin surface before local enforcement loop is solid
- Platform claims beyond what current OS and engine really enforce
