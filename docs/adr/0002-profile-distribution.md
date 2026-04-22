# ADR 0002: Profile Distribution and Signing

## Decision

Profiles distributed across a team use ed25519 signatures embedded in a `[signature]`
block within the existing profile format. Distribution happens via filesystem path or
HTTPS URL. Rampart verifies signatures before loading; the local profile store remains
authoritative.

## Why

- Profiles are already structured data in policy-core's schema. Adding a `[signature]`
  block is additive and backward-compatible — unsigned profiles still load unless the
  workspace is configured to require signatures.
- ed25519 is fast, small (32-byte public key, 64-byte signature), and widely supported
  via `ed25519-dalek`.
- File-system and HTTPS distribution cover the realistic team cases (shared network
  drive, internal URL, git-committed URL in a team wiki) without requiring Rampart to
  operate or depend on a hosted service.
- The org policy mechanism (Phase 4 §3) reuses the same signed-profile format, so one
  verification path handles both team profiles and org floors.

## Alternatives considered

- **JWT / JWS** — more complex, carries unnecessary claims overhead for a static file format.
- **GPG** — operational complexity (keyring management, trust model) is too high for
  typical dev teams; ed25519 key pairs are simpler to generate and distribute.
- **Hash pinning without signatures** — detects corruption but not tampering; a bad actor
  who can change the file can change the pinned hash too.

## Consequences

- policy-core gains a `verify_signature` function and a dependency on `ed25519-dalek`.
- Profile loading in rampartd verifies the signature (if present) and returns an error
  variant that propagates to the preflight diagnostic.
- Teams that do not use signing are unaffected; the `[signature]` block is optional.
- Key rotation requires re-signing and redistributing the profile. No revocation
  mechanism in Phase 4 — compromised keys require immediate re-signing of all profiles.
