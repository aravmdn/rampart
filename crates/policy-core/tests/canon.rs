use policy_core::{
    compile_policy, validate_policy_against_capabilities, AgentTool, AuditEvent, AuditEventKind,
    AuditOutcome, CapabilitySupport, CompiledPolicy, DefaultAction, EngineCapabilitySnapshot,
    FilesystemCapabilitySnapshot, FilesystemPolicy, NetworkCapabilitySnapshot, NetworkPolicy,
    Policy, ProcessCapabilitySnapshot, ProcessPolicy, Profile, Session, SessionStatus,
    ViolationEvent, ViolationKind,
};
use serde_json::json;

#[test]
fn stable_agent_tool_name_serialization() {
    let value = serde_json::to_value(AgentTool::Codex).expect("serialize");

    assert_eq!(value, json!({ "kind": "codex" }));
}

#[test]
fn compile_policy_returns_engine_neutral_shape() {
    let policy = Policy {
        filesystem: FilesystemPolicy {
            readable_roots: vec!["src".into(), "docs".into()],
            writable_roots: vec!["src".into()],
            blocked_roots: vec![".git".into()],
        },
        network: NetworkPolicy {
            default_action: DefaultAction::Deny,
            allowed_hosts: vec!["api.openai.com".into()],
            blocked_hosts: vec!["example.com".into()],
        },
        process: ProcessPolicy {
            default_action: DefaultAction::Deny,
            allowed_commands: vec!["git".into()],
            blocked_commands: vec!["powershell".into()],
        },
    };

    let compiled = compile_policy(&policy).expect("compile");
    let snapshot = EngineCapabilitySnapshot {
        engine_name: "windows-runtime".into(),
        engine_version: Some("0.1.0".into()),
        platform: "windows".into(),
        filesystem: FilesystemCapabilitySnapshot {
            enforcement: CapabilitySupport::Supported,
            observation: CapabilitySupport::Supported,
            temporary_file_coverage: CapabilitySupport::Limited,
            atomic_rename_coverage: CapabilitySupport::Limited,
        },
        network: NetworkCapabilitySnapshot {
            enforcement: CapabilitySupport::Supported,
            observation: CapabilitySupport::Supported,
            proxy_awareness: CapabilitySupport::Limited,
        },
        process: ProcessCapabilitySnapshot {
            enforcement: CapabilitySupport::Supported,
            observation: CapabilitySupport::Supported,
            termination: CapabilitySupport::Supported,
        },
    };

    validate_policy_against_capabilities(&policy, &snapshot).expect("supported");

    let expected = CompiledPolicy {
        filesystem: policy.filesystem.clone().into(),
        network: policy.network.clone().into(),
        process: policy.process.clone().into(),
    };

    assert_eq!(compiled, expected);
}

#[test]
fn canonical_entities_construct_cleanly() {
    let compiled_policy = CompiledPolicy {
        filesystem: FilesystemPolicy {
            readable_roots: vec!["src".into()],
            writable_roots: vec!["src".into()],
            blocked_roots: vec![],
        }
        .into(),
        network: NetworkPolicy {
            default_action: DefaultAction::Deny,
            allowed_hosts: vec![],
            blocked_hosts: vec![],
        }
        .into(),
        process: ProcessPolicy {
            default_action: DefaultAction::Deny,
            allowed_commands: vec![],
            blocked_commands: vec![],
        }
        .into(),
    };

    let _profile = Profile {
        id: "nextjs".into(),
        name: "Next.js".into(),
        description: Some("Default web profile".into()),
        extends: None,
        policy: Policy {
            filesystem: FilesystemPolicy {
                readable_roots: vec!["src".into()],
                writable_roots: vec!["src".into()],
                blocked_roots: vec![],
            },
            network: NetworkPolicy {
                default_action: DefaultAction::Deny,
                allowed_hosts: vec![],
                blocked_hosts: vec![],
            },
            process: ProcessPolicy {
                default_action: DefaultAction::Deny,
                allowed_commands: vec!["git".into()],
                blocked_commands: vec![],
            },
        },
    };

    let _session = Session {
        id: "session-1".into(),
        project_path: "C:\\projects\\rampart".into(),
        agent_tool: AgentTool::Codex,
        profile_id: "nextjs".into(),
        compiled_policy,
        status: SessionStatus::Running,
        started_at_ms: 100,
        ended_at_ms: None,
    };

    let violation = ViolationEvent {
        session_id: "session-1".into(),
        sequence: 1,
        occurred_at_ms: 101,
        kind: ViolationKind::Filesystem,
        action: "read".into(),
        target: ".env".into(),
        rule_id: "fs.read.project-root-only".into(),
        rule_label: "Read access limited to selected project root.".into(),
        reason: "outside readable roots".into(),
        platform_note: Some("Windows runtime proof fixture.".into()),
    };

    let _audit = AuditEvent {
        session_id: "session-1".into(),
        sequence: 2,
        occurred_at_ms: 102,
        kind: AuditEventKind::ViolationRecorded,
        outcome: AuditOutcome::Blocked,
        message: "blocked secret read".into(),
        violation: Some(violation),
    };
}

#[test]
fn desktop_profile_presets_are_windows_first_and_compile() {
    let presets = policy_core::desktop_profile_presets(r"C:\projects\rampart");

    assert_eq!(presets.len(), 2, "expected safe and strict presets");
    assert_eq!(presets[0].id, "windows-safe");
    assert_eq!(presets[1].id, "windows-strict");
    assert_eq!(presets[1].extends.as_deref(), Some("windows-safe"));

    for preset in presets {
        preset.validate().expect("preset should validate");
        compile_policy(&preset.policy).expect("preset should compile");
    }
}
