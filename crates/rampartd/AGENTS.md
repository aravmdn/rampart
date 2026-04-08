# Agent instructions (scope: this directory and subdirectories)

## Scope
- This file applies to `crates/rampartd/`.
- This module owns local orchestration and process supervision.

## Responsibilities
- validate launch requests
- supervise sessions
- call engine adapters
- persist and stream session events
- expose a narrow local API to the desktop app

## Conventions
- Fail loudly with actionable diagnostics.
- Keep engine-specific logic behind adapter boundaries.
- Treat platform capability detection as first-class behavior.

## Do not
- Do not move UI concerns into this crate.
- Do not bypass policy validation before launch.
