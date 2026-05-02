# 02 PHASE STATUS
last updated: 2026-05-02 (routine 13 — mapping-layer edge case tests)

## DONE — Phase 1
agent adapters + preflight in rampartd · capability warnings before launch · launcher/session console split · terminal-first UX preserved

## DONE — Phase 2
SessionHistoryRecord w/ capability snapshot · ViolationExplanation (policy reason + platform limitation + remediation hint) · AuditEventCategory + AuditEventKind variants · HistoryList + HistoryDetailPanel in shared-ui · agent_profile_presets() (ClaudeCode/Codex/Aider) · profiles_for_agent() + launch_context() filtered by agent · ProfileDetail/loadProfile/saveProfile in contracts + both clients · ProfileEditorPanel (textarea lists + default-action toggles) · PolicyEditorView + PolicySuggestion types · profile editor view in App.tsx · policy refinement flow (Adjust policy → pre-populated editor)

## CURRENT — Phase 3 (MVP gate)
real OS-level blocking required. nothing ships until end-to-end block works.

done so far:
- engine-windows crate: WindowsEnforcer + WindowsJob (Job Object process tree containment)
- LocalProcessRunner wires Job Objects on Windows (spawn → assign → kill-on-close)
- main.rs uses WindowsEnforcer on Windows, GreywallAdapter elsewhere
- threat-model.md: honest accidental-overreach framing, current gaps, WSL2/AppContainer seam
- set_process_low_integrity(): post-spawn token integrity reduction to Low (S-1-16-4096); denies writes to Medium+ paths (user profile, system dirs); best-effort, non-fatal on failure
- patch_project_low_integrity_label(): sets Low mandatory label on project root SACL (SetNamedSecurityInfoW + AddMandatoryAce); called in spawn_session before launch; resolves the known gap where agent couldn't write to project root
- WfpNetworkGuard: full per-app-ID outbound BLOCK filter; SearchPathW resolves short command to full path; QueryDosDeviceW converts to NT device path; FWPM_CONDITION_ALE_APP_ID filter on ALE connect V4+V6; dynamic WFP session (FWPM_SESSION_FLAG_DYNAMIC) auto-removes filters on Drop; best-effort, non-fatal on failure
- EtwAuditProvider: registers Rampart ETW provider (GUID 7E5A6B4C-F3D2-4A81-9B62-C1E0A4B8D7F6); EventWriteString on every AuditEvent from ingest_raw_event, launch_session, stop_session; capturable with logman
- capability snapshot: process=Supported, filesystem=Limited, network=Limited

runtime validation (2026-04-28, session 8):
- Test 1 (Job Object containment): PASS — job handle held, child processes terminate on session close
- Test 2 (Low Integrity token): PASS — spawned process runs at Low integrity; denies writes to Medium+ paths
- Test 3 (Project root SACL): PASS — project root has Low mandatory label; agent can write there
- Test 4 (WFP outbound block): PASS — app-ID filter blocks outbound TCP; dynamic session auto-cleans on close
- Test 5 (ETW audit): PASS — logman trace captures Rampart provider GUID events for every AuditEvent
- Test 6 (End-to-end UI launch): PARTIAL — app launches cleanly (14MB binary, window shown, preflight passes for claude-code); violation streaming is unsupported in Windows Native mode (blocked at OS level but events don't surface in session console); UI automation couldn't reliably trigger launch button to confirm agent spawn

known limitation: violation events blocked at OS level by WFP/Job/SACL do not stream back into session console in Windows Native mode. The enforcement works; the feedback loop is silent. Documented; Phase 4+ to address if needed.

remaining:
- AppContainer seam for Phase 5

done (fast-follow):
- WSL2 stronger isolation mode: IsolationMode enum on Session, detect_wsl2() + preflight diagnostic, Wsl2 launch path (wsl --cd), Wsl2Enforcer with Supported capabilities, IsolationMode in TypeScript contracts

## ## DONE — Phase 4.1 + 4.2 (2026-04-29)

Phase 4.1 (signed profile distribution) — COMPLETE:
- ProfileSignature + SignatureStatus + SignError in policy-core
- verify_signature() + sign_profile() using ed25519-dalek in policy-core
- signature field on Profile; signature_status on ProfileDetail and ProfileSummary
- sign_profile Tauri command + DaemonApi.signProfile
- UI badge: "[signed]" / "[signature invalid]" in profile picker
- HTTPS profile fetch: fetch_remote_profile(url, cache_dir) with disk cache fallback
- load_remote_profile Tauri command + DaemonApi.loadRemoteProfile
- Preflight diagnostic for Invalid signature: Fail severity blocks launch

Phase 4.2 (centralized audit sync) — COMPLETE (manual flush; no background worker):
- SyncConfig (endpointUrl, token, stripPaths) persisted in local state
- AuditQueueEntry: events enqueued in persist_history_record when sync is configured
- sync_audit_events: batch 500 unsent → POST to endpoint with Bearer token; marks sent
- configure_sync / get_sync_status / sync_audit_events Tauri commands
- SyncConfig + SyncStatus in TypeScript contracts + both clients
- Sync settings panel in launcher (URL, token, strip paths, Save, Sync Now, status)

## DONE — Phase 4.3 (2026-05-01)

Phase 4.3 (org settings) — COMPLETE:
- OrgPolicy struct in policy-core (subset of Profile fields: allowed_paths, blocked_paths, default_action, allowed_hosts, blocked_hosts)
- OrgPolicyScope struct (agent_type glob + project_path_prefix glob) — scope matching via org_policy_applies()
- resolve_effective_policy(local, org) in policy-core — strict merge (org floors win; most restrictive)
- org_policy_url + cached_org_policy fields on PersistedState in rampartd
- configure_org_policy_url / fetch_org_policy / current_org_policy Tauri commands
- run_preflight uses resolve_effective_policy to merge org floor into local profile before capability assessment
- PreflightDiagnostic.from_org_policy field (bool) — marks diagnostics sourced from org policy
- "Org policy floor active" preflight notice when org policy applies to current agent + project
- TypeScript: OrgPolicy + OrgPolicyScope types in contracts.ts; three new DaemonApi methods in both clients
- PreflightDiagnostic.fromOrgPolicy in contracts.ts
- UI: org policy floor badge in launcher preflight panel + [from org policy] inline tags on diagnostics

Phase 4 complete (2026-05-01):
- Background sync worker: `drain_audit_queue(store)` free function; `trigger_background_sync()` spawns thread on `stop_session`; `sync_audit_events` delegates to it

Scope doc: `docs/phase4-scope.md` · ADR: `docs/adr/0002-profile-distribution.md`

## DONE — Phase 5 (partial, 2026-05-01)

Phase 5 polish + WFP violation streaming:
- Dev artifact (TASK3_API_NAMES) removed from UI
- Org policy URL config UI added to launcher
- Live session event polling (3s interval on active sessions)
- WFP event monitor: FwpmNetEventSubscribe0-based real-time blocked connection notifications surface in session console while session is live; drained and persisted at stop_session
- NetworkCapabilitySnapshot.observation = Limited (was Unsupported)

Done (2026-05-01):
- Headless CLI: apps/cli/ crate (binary: rampart); `rampart run --agent <id> --profile <id> --project <path> [--wsl2]`
  - Preflight → stderr; audit JSONL → stdout; Ctrl+C clean shutdown; all enforcement via RampartService
- `rampart list-profiles --agent <id> --project <path>` subcommand
- is_session_running() on LocalProcessRunner + RampartService

Done (2026-05-02 — routine #8):
- Per-agent capability matrices: Cursor, Copilot, Goose, OpenCode, GeminiCli presets in agent_profile_presets()
  - Each agent gets standard (API-allowed) + strict (network-denied) profiles
  - cursor.standard allows cursor.sh endpoints; copilot.standard allows GitHub API; gemini.standard allows googleapis.com
  - 12 new Rust unit tests in preset_tests module

Remaining Phase 5 work:
- AppContainer isolation mode (seam clean; deferred — most complex)
- WFP monitor privilege validation (runtime; unknown if monitor works at Medium integrity)

## FUTURE — Phase 5 (continued)
AppContainer (principled, complex per-agent work; seam preserved in engine adapter arch)
CLI runtime validation (requires VS Dev Shell; not testable in scheduled routine env)
