# Task 2 sync asks

Need exact answers before broader cross-crate sharing:

1. IDs: keep `AgentId`, `ProfileId`, `SessionId`, `ProjectId` as strong newtypes over strings, or flatten to string aliases?
2. Profiles: is `id`, `name`, `description`, `agent_id`, `source`, `updated_at` enough for desktop and policy-core?
3. Events: keep kinds to `session_lifecycle`, `process_lifecycle`, `filesystem_access`, `network_access`, `diagnostic`, or add `alert` and `violation` now?
4. Time: wire timestamps as RFC3339 strings, or epoch millis?
5. Capabilities: keep `supported|limited|unsupported|unavailable`, or use another enum?
6. Stop API: single-session only now, or reserve bulk stop in shared contract?

Observed parallel worktree mismatch inside `crates/rampartd`:
- `src/main.rs` currently references `LaunchRequest`
- `tests/blocked_action_loop.rs` currently references `SessionHarness`
- that test also uses `event_type` string values like `session.launched` and `violation.blocked`

Need main decision:
- keep those names and adapt this contract
- or keep this narrower API contract as shared boundary and treat harness/types as internal test fixtures
