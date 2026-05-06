# AGENTS.md — shared-ui

Scope: this directory and its subdirectories. Defers to the root AGENTS.md and CLAUDE.md
for repo-wide conventions.

## What this module owns

Reusable React components and types consumed by desktop app: UI primitives (buttons, panels, lists), design tokens, session console component with violation display and capability warnings, history list and detail panels, profile editor with policy refinement flow.

## How to build / run

```sh
pnpm build:shared-ui
pnpm --filter @rampart/shared-ui exec vitest run  # if testing
```

## How to test

```sh
pnpm --filter @rampart/shared-ui exec vitest run
```

## Key constraints

- Exports consumed via `@rampart/shared-ui` alias defined in `vite.config.ts`.
- No Tauri-specific code; all integration with daemon APIs happens in `apps/desktop/`.
- Mock daemon client must implement `DaemonApi` contract; keep in sync with `contracts.ts`.
- Reusable by future desktop frontends (e.g., alternative UI shell).
