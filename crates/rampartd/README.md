# rampartd

Narrow local daemon API contract.

Scope:
- request/response/event/error types
- stable product-level fields desktop can mock
- no UI logic
- no engine-specific launch/config surface

Current contract surface:
- `detect_capabilities`
- `list_profiles`
- `launch_session`
- `stop_session`
- `stream_session_events`

Contract rules:
- capability detection first-class
- explicit support levels: `supported`, `limited`, `unsupported`, `unavailable`
- launch takes `agent_id`, `project_dir`, `profile_id`
- stop handles already-stopped as response outcome, not generic crash
- stream envelopes separate `kind` from `payload`
- errors always typed and actionable

Open sync points with Task 2:
- ID newtypes vs aliases
- profile summary minimum fields
- event taxonomy final set
- timestamp wire format
- capability status enum
- stop-all reservation vs single-session only
