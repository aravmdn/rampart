# Agent instructions (scope: this directory and subdirectories)

## Scope
- This file applies to `apps/desktop/`.
- This module owns the local desktop UX for Rampart.

## Responsibilities
- project selection
- agent and profile selection
- session lifecycle UI
- live event and violation display
- settings and diagnostics

## Conventions
- Keep process launch and policy compilation out of the UI layer.
- Prefer simple, explicit UI state over hidden orchestration.
- Surface platform and engine limitations clearly.

## Do not
- Do not embed sandbox policy logic directly in UI components.
- Do not make the UI the source of truth for enforcement state.
