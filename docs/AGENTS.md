# AGENTS.md — docs

Scope: this directory and its subdirectories. Defers to the root AGENTS.md for repo-wide conventions.

## What this module owns

Architecture documentation (layers, modules, IPC boundary, design decisions), threat model, roadmap, product definition, onboarding guides, decision records (ADRs).

## How to build / run

Docs are plain Markdown. No build step required. Read them in place or in a Markdown viewer.

## How to test

No automated tests. Manual review for:
- Accuracy against current codebase (run after major refactors)
- Completeness (new features should add or update relevant docs)
- External reference links (spot-check that links are valid and point to intended sources)

## Key constraints

- Do not commit private working notes or vault content; this is public documentation.
- When architecture or roadmap changes, update docs/ alongside code changes.
- Threat model is honest about enforcement gaps and known limitations (e.g., WFP violation silence in Windows Native mode, AppContainer seam).
- No marketing claims about unsupported isolation modes or future phases in MVP docs.
