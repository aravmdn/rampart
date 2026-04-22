# Phase 4 Scope — Team and Distribution Features

Last updated: 2026-04-22

This document breaks down Phase 4's three areas into concrete implementation tasks.
Phase 4 begins after Phase 3 runtime validation closes the MVP gate.

---

## 1. Signed Profile Distribution

### Problem

Teams want to distribute a standard profile to every developer without relying on
copy-paste or hoping individuals don't accidentally loosen rules. The profile must
be tamper-evident so Rampart can refuse to load a locally modified copy.

### Design

**Profile format** — profiles are already structured data (policy-core schema).
Phase 4 adds an optional `[signature]` block at the top level:

```toml
[signature]
signer    = "team@example.com"
algorithm = "ed25519"
value     = "<base64-encoded signature over canonical profile bytes>"
```

The signature covers the canonical serialization of the profile (excluding the
`[signature]` block itself). Rampart verifies before loading; a tampered or
unsigned-but-required profile is rejected with a clear preflight diagnostic.

**Key management** — Phase 4 ships with:
- `rampart profile sign <file> --key <ed25519-private-key>` (CLI subcommand, Phase 5)
- For Phase 4: a simpler `sign_profile` daemon command callable from the desktop
  so a team lead can sign from the UI without a separate CLI binary

**Distribution channels** (in priority order):
1. File-system path — teams drop the signed profile into a shared directory or
   mounted drive; Rampart resolves it by path just like a local profile.
2. HTTPS URL — Rampart fetches and caches a signed profile from an HTTPS endpoint.
   Cache is refreshed on session start (background, non-blocking on failure).
3. git ref — future (Phase 5). Resolving a profile from a git URL reuses the
   fetch mechanism but adds commit-pinning.

### Implementation tasks

| Task | Crate/Package | Notes |
|------|--------------|-------|
| Add `[signature]` block to profile schema | policy-core | Optional field; absent = unsigned |
| `verify_signature()` in policy-core | policy-core | ed25519-dalek; error variant for invalid/missing |
| `sign_profile` Tauri command | main.rs | Takes profile path + private key bytes; returns signed profile bytes |
| Signature check in `load_profile` | rampartd | Verify before returning; propagate error to preflight |
| HTTPS profile fetch + disk cache | rampartd | `reqwest` (already in dep tree); cache in rampart local state dir |
| Preflight diagnostic for signature failure | rampartd | Distinct `PreflightDiagnostic` variant |
| UI: signature status badge in profile picker | shared-ui | Green lock / yellow warning / red blocked |

---

## 2. Centralized Audit Sync

### Problem

Teams want to see what all agent sessions did, not just the local history on each
developer's machine. This means shipping local audit events to a team endpoint.

### Design

**Push model, local-first** — Rampart never blocks session launch on sync success.
Sync is a background append-only push that retries on failure. The local audit log
is authoritative; the remote endpoint is a mirror.

**Event batching** — events accumulate in a local send queue (SQLite table or
append-only log file). A background worker drains the queue in batches of up to
500 events, sending to a configured endpoint URL.

**Endpoint contract**

```
POST /api/v1/audit/events
Authorization: Bearer <team-token>
Content-Type: application/json

{
  "session_id": "...",
  "events": [ { ...AuditEvent... } ]
}
```

The team endpoint is configured per-workspace in rampart local state (not committed
to git). The desktop exposes a settings page to enter and validate the endpoint URL
and token.

**Privacy** — file path values in events are not truncated in the local log, but the
sync config supports a `strip_paths: true` option that replaces filesystem path
values with `<redacted>` before sending. Default is off.

### Implementation tasks

| Task | Crate/Package | Notes |
|------|--------------|-------|
| Audit send queue table in local state | rampartd | Append-only; track `sent_at` per event |
| Background sync worker | rampartd | Tokio task; respects backoff on 429/5xx |
| Sync config: endpoint URL + token + strip_paths | rampartd | Persisted in workspace state, not profile |
| `configure_sync` / `get_sync_status` Tauri commands | main.rs | Returns queue depth, last success, last error |
| Sync settings panel in desktop | shared-ui + App.tsx | URL input, token input, status badge, queue depth |
| `strip_paths` redaction pass | rampartd | Applied before batching if enabled |

---

## 3. Org Settings

### Problem

Org admins want to enforce minimum policy requirements (e.g., network always
blocked for certain project types) without depending on developers to select the
right profile.

### Design

**Above the local enforcement loop** — org settings augment, not replace, the
local profile. They define a minimum policy floor: if an org rule requires network
blocked, the local profile cannot loosen that rule even if the developer tries.

**Floor merge** — `resolve_effective_policy(local_profile, org_policy)` returns
a merged policy that takes the stricter value for each dimension. This happens in
policy-core and is transparent to the enforcement engine.

**Org policy distribution** — an org policy is itself a signed profile (using the
same signature mechanism from §1) fetched from the team endpoint. This reuses the
HTTPS fetch + cache path and avoids a separate config format.

**Scope** — org settings scoped to: all sessions, specific agent types, or specific
project directory patterns (glob match on project path).

### Implementation tasks

| Task | Crate/Package | Notes |
|------|--------------|-------|
| `OrgPolicy` struct (subset of Profile fields + scope) | policy-core | Reuses existing rule types |
| `resolve_effective_policy()` | policy-core | Strict merge: take most restrictive value |
| Org policy fetch via existing HTTPS profile channel | rampartd | Special-cased `org_policy` config key |
| Scope matching: agent type + project path glob | policy-core | Simple fnmatch-style matching |
| `preflight_check` includes org policy floor in explanation | rampartd | Explain which rules come from org vs. local |
| UI: show "org policy floor active" in capability panel | shared-ui | Informational badge; no edit allowed |

---

## Implementation order within Phase 4

1. Signed profile distribution (§1) — no backend dependency; ships value immediately.
2. Centralized audit sync (§2) — self-contained background worker; teams can start collecting data.
3. Org settings (§3) — depends on §1 (org policy is a signed profile) and optionally §2 (sync).

---

## What Phase 4 does NOT include

- A hosted backend. Teams run their own endpoint, or we provide a reference server
  in a companion repo. Rampart core ships no cloud dependency in Phase 4.
- SSO or identity federation. Token-based auth is sufficient for Phase 4 scope.
- A management dashboard. Audit viewing happens in whatever tool the team points
  the sync endpoint at (S3 + Athena, a simple web UI, etc.).
- Real-time streaming. Batch push with retry is enough. Streaming is Phase 5 if needed.
