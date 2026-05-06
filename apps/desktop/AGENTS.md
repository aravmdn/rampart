# AGENTS.md — desktop

Scope: this directory and its subdirectories. Defers to the root AGENTS.md and CLAUDE.md
for repo-wide conventions.

## What this module owns

Local GUI shell (Tauri) for project/agent/profile selection, live session console with violation explanations and capability warnings, session history view, profile editor with policy refinement flow, and IPC client mapping between TypeScript and Rust daemon APIs.

## How to build / run

```sh
pnpm install
pnpm build:shared-ui
pnpm dev:desktop  # runs Tauri dev server; watches src/ and src-tauri/
```

## How to test

```sh
pnpm install
pnpm --filter @rampart/desktop exec vitest run
pnpm --filter @rampart/desktop exec tsc --noEmit
```

## Key constraints

- Frontend talks to daemon only via Tauri invoke commands; see `tauriDaemonClient.ts` for mapping layer.
- IPC boundary requires Serialize/Deserialize on Rust types; TypeScript types in `contracts.ts` must match `tauriDaemonClient.ts` mapping.
- Mock client in `mockDaemonClient.ts` must implement every method in `DaemonApi`; keep in sync when contract changes.
- `shared-ui` components consume via `@rampart/shared-ui` alias defined in `vite.config.ts`.
- Test environment is jsdom; `App.test.tsx` is the integration regression check.
