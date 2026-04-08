# ADR 0001: Workspace layout

## Status
Accepted

## Decision
Rampart will start with a split repository structure:
- `apps/desktop` for the local UI
- `crates/rampartd` for daemon and orchestration logic
- `crates/policy-core` for policy and validation logic
- `crates/engine-greywall` for the initial enforcement engine adapter
- `packages/shared-ui` for reusable UI pieces
- `docs` for architecture and decision records

## Rationale
- Keeps UI, orchestration, policy, and engine concerns separate
- Supports a local-first desktop product without embedding system logic in the UI
- Preserves a future path to alternate enforcement engines, which is required for a Windows-first roadmap

## Consequences
- More structure up front
- Clearer ownership and lower coupling as implementation starts
