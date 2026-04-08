# Agent instructions (scope: this directory and subdirectories)

## Scope
- This file applies to `crates/policy-core/`.
- This module owns policy definitions and validation.

## Responsibilities
- policy schema
- profile schema
- validation
- policy compilation inputs
- normalized event types shared across modules

## Conventions
- Keep user-facing policy concepts stable and engine-agnostic.
- Prefer explicit validation errors over permissive fallback behavior.

## Do not
- Do not depend on UI code.
- Do not hardcode assumptions that belong to one enforcement engine only.
