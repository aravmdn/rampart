# rampartd API contract

Transport-agnostic local contract. JSON examples show wire shape desktop can mock now.

## `detect_capabilities`

Request:

```json
{
  "refresh": true
}
```

Response:

```json
{
  "snapshot_id": "cap_01",
  "detected_at": "2026-04-09T10:00:00Z",
  "engine": "greywall",
  "engine_version": "0.3.0",
  "platform": "windows",
  "filesystem": "supported",
  "network_egress": "limited",
  "process_execution": "supported",
  "process_termination": "supported",
  "event_stream": "supported",
  "limitations": [
    "network proxy fallback only"
  ]
}
```

## `list_profiles`

Request:

```json
{
  "agent_id": "codex",
  "project_dir": "C:\\projects\\rampart"
}
```

Response:

```json
{
  "profiles": [
    {
      "id": "profile_nextjs",
      "name": "Next.js",
      "description": "Default-deny profile for Next.js repos.",
      "agent_id": "codex",
      "source": "builtin",
      "updated_at": "2026-04-09T10:00:00Z"
    }
  ]
}
```

## `launch_session`

Request:

```json
{
  "project_dir": "C:\\projects\\rampart",
  "agent_id": "codex",
  "profile_id": "profile_nextjs",
  "max_blocked_actions": 5
}
```

Response:

```json
{
  "session": {
    "id": "sess_01",
    "state": "launching",
    "started_at": "2026-04-09T10:00:03Z",
    "ended_at": null,
    "agent_id": "codex",
    "project_dir": "C:\\projects\\rampart",
    "profile_id": "profile_nextjs",
    "blocked_action_count": 0,
    "terminated_by_user": false
  },
  "capabilities": {
    "snapshot_id": "cap_01",
    "detected_at": "2026-04-09T10:00:00Z",
    "engine": "greywall",
    "engine_version": "0.3.0",
    "platform": "windows",
    "filesystem": "supported",
    "network_egress": "limited",
    "process_execution": "supported",
    "process_termination": "supported",
    "event_stream": "supported",
    "limitations": [
      "network proxy fallback only"
    ]
  }
}
```

## `stop_session`

Request:

```json
{
  "session_id": "sess_01"
}
```

Response:

```json
{
  "session_id": "sess_01",
  "outcome": "stopping",
  "requested_at": "2026-04-09T10:00:15Z"
}
```

## `stream_session_events`

Request:

```json
{
  "session_id": "sess_01",
  "after_sequence": 12,
  "follow": true
}
```

Stream item:

```json
{
  "event_id": "evt_13",
  "session_id": "sess_01",
  "sequence": 13,
  "occurred_at": "2026-04-09T10:00:16Z",
  "severity": "warn",
  "kind": "filesystem_access",
  "payload": {
    "action": "read",
    "decision": "blocked",
    "path": "C:\\Users\\alice\\.ssh\\id_rsa",
    "pid": 4821,
    "rule": "deny_read_outside_project"
  }
}
```

## Error shape

```json
{
  "code": "profile_not_found",
  "message": "profile missing",
  "actionable": "select different profile",
  "retryable": false,
  "details": {
    "profile_id": "profile_nextjs"
  }
}
```
