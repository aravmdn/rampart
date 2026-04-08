# Agent instructions (scope: this directory and subdirectories)

## Scope
- This file applies to `crates/engine-greywall/`.
- This module owns the adapter around the initial enforcement engine.

## Responsibilities
- engine discovery
- version checks
- policy translation
- process launch integration support
- raw output parsing into structured Rampart events

## Conventions
- Be precise about unsupported capabilities.
- Isolate greywall-specific assumptions in this module.
- Preserve a clean contract for the daemon.

## Do not
- Do not leak raw engine quirks into higher-level product APIs unless necessary.
- Do not assume feature parity across platforms.
