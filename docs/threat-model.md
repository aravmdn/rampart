# Threat Model

Rampart exists because AI coding agents can inherit broad user permissions and touch data they should not reach.

## Assets

- Source code
- Local secrets and credentials
- SSH keys and shell history
- Network access
- Session history and audit data

## Main threats

- An agent reads files outside the intended project scope
- An agent writes to sensitive paths or exfiltrates local data
- An agent makes outbound network calls that violate policy
- A buggy or compromised agent attempts process abuse or persistence
- A user mistakes a prompt instruction for a real security boundary

## Security stance

- Enforcement must happen outside the model prompt
- Policies should be deterministic and explicit
- Default-deny is the baseline
- Capability limits must be visible to the user before launch
- Logs should explain blocked behavior without exposing private secrets

## Product boundary

Rampart is a policy and visibility layer around agent execution. It is not a generic observability platform and not a prompt firewall.
