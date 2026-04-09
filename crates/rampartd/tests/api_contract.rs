use rampartd::{
    ApiError, ApiErrorCode, CapabilitySupport, DetectCapabilitiesResponse, LaunchSessionRequest,
    ProfileId, SessionEventEnvelope, SessionId,
};
use std::collections::BTreeMap;

#[test]
fn detect_capabilities_response_uses_explicit_domain_support() {
    let response = DetectCapabilitiesResponse {
        snapshot_id: "cap_01".into(),
        detected_at: "2026-04-09T10:00:00Z".into(),
        engine: "greywall".into(),
        engine_version: Some("0.3.0".into()),
        platform: "windows".into(),
        filesystem: CapabilitySupport::Supported,
        network_egress: CapabilitySupport::Limited,
        process_execution: CapabilitySupport::Supported,
        process_termination: CapabilitySupport::Supported,
        event_stream: CapabilitySupport::Supported,
        limitations: vec!["network proxy fallback only".into()],
    };

    let json = serde_json::to_value(response).unwrap();

    assert_eq!(json["filesystem"], "supported");
    assert_eq!(json["network_egress"], "limited");
    assert!(json["limitations"].is_array());
}

#[test]
fn launch_session_request_requires_agent_project_and_profile_ids() {
    let request = LaunchSessionRequest {
        project_dir: r"C:\projects\rampart".into(),
        agent_id: "codex".into(),
        profile_id: ProfileId::new("profile_nextjs"),
        max_blocked_actions: Some(5),
    };

    let json = serde_json::to_value(request).unwrap();

    assert_eq!(json["agent_id"], "codex");
    assert_eq!(json["profile_id"], "profile_nextjs");
    assert!(json.get("policy").is_none());
}

#[test]
fn session_event_envelope_separates_kind_from_payload() {
    let envelope = SessionEventEnvelope::diagnostic(
        SessionId::new("sess_01"),
        7,
        "2026-04-09T10:00:05Z",
        "engine warmup complete",
    );

    let json = serde_json::to_value(envelope).unwrap();

    assert_eq!(json["kind"], "diagnostic");
    assert_eq!(json["sequence"], 7);
    assert_eq!(json["payload"]["message"], "engine warmup complete");
}

#[test]
fn api_error_is_typed_and_actionable() {
    let error = ApiError {
        code: ApiErrorCode::ProfileNotFound,
        message: "profile missing".into(),
        actionable: Some("select different profile".into()),
        retryable: false,
        details: BTreeMap::new(),
    };

    let json = serde_json::to_value(error).unwrap();

    assert_eq!(json["code"], "profile_not_found");
    assert_eq!(json["retryable"], false);
    assert_eq!(json["actionable"], "select different profile");
}
