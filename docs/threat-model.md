# Rampart threat model

## In scope
- AI coding agents reading sensitive local files
- AI coding agents writing outside intended workspace boundaries
- AI coding agents making unexpected network requests
- Missing or unclear audit evidence around agent behavior

## Trust boundaries
- User desktop app
- Local daemon
- Enforcement engine
- Child AI agent process
- Local filesystem and developer credentials
- Local network access and outbound destinations

## Main security assumptions
- The enforcement engine is the source of truth for what was technically blocked or allowed.
- Product policy must not imply capabilities the current platform cannot enforce.
- Logs and violation data are security-sensitive and should be treated as local operational records.

## Initial risks
- Windows enforcement strategy not matching product expectations
- Platform capability mismatch
- Upstream engine dependency risk
- Misleading UX that overstates protection
- Policy complexity overwhelming users

## Immediate follow-up
- Add concrete abuse cases
- Add per-platform capability matrix
- Add assumptions for secret exposure and environment handling
