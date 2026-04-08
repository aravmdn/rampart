# Agent instructions (scope: this directory and subdirectories)

## Scope
- This file applies to `packages/shared-ui/`.
- This module owns reusable UI primitives and shared Rampart domain components.

## Responsibilities
- shared visual primitives
- event display components
- profile and policy presentation components
- design tokens once a design system exists

## Conventions
- Keep shared components presentational where possible.
- Avoid embedding daemon or engine assumptions directly in reusable UI.

## Do not
- Do not duplicate feature-specific state management here.
