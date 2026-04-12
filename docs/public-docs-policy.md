# Public Documentation Policy

Rampart keeps a small public documentation set in git so contributors and users can understand the product without exposing private working notes.

## Public by default

- `README.md`
- `AGENTS.md`
- `docs/architecture.md`
- `docs/threat-model.md`
- `docs/business-model.md`
- `docs/roadmap.md`
- `docs/adr/*.md`
- `docs/public-docs-policy.md`

## Private-only docs

- `docs/PRD.md`

## Private and local-only

- `AGENTS.md`
- prompt files
- task notes
- sync notes
- personal planning notes
- any file that contains secrets, credentials, private strategy, or founder-specific working context

## Rules

- Do not commit private notes into the public repo.
- Do not store secrets, keys, passwords, tokens, or credentials in tracked files.
- Keep implementation guidance in public docs at a product level, not as private operator instructions.
- If a doc becomes too internal for public release, move its sensitive parts out of git instead of hiding the problem in a comment.
