use policy_core::{
    compile_policy, validate_policy_against_capabilities, AgentTool, CapabilitySupport,
    DefaultAction, EngineCapabilitySnapshot, FilesystemCapabilitySnapshot, FilesystemPolicy,
    NetworkCapabilitySnapshot, NetworkPolicy, Policy, ProcessCapabilitySnapshot, ProcessPolicy,
    Profile, ValidationErrorCode,
};

#[test]
fn reject_empty_custom_agent_tool_id() {
    let result = AgentTool::Custom {
        id: "   ".into(),
        display_name: None,
    }
    .validate();

    let errors = result.expect_err("must fail");
    assert_eq!(errors.items[0].code, ValidationErrorCode::InvalidValue);
    assert_eq!(errors.items[0].field, "agent_tool.id");
}

#[test]
fn reject_duplicate_filesystem_roots() {
    let result = Profile {
        id: "base".into(),
        name: "Base".into(),
        description: None,
        extends: None,
        policy: Policy {
            filesystem: FilesystemPolicy {
                readable_roots: vec!["src".into(), "src".into()],
                writable_roots: vec![],
                blocked_roots: vec![],
            },
            network: NetworkPolicy {
                default_action: DefaultAction::Deny,
                allowed_hosts: vec![],
                blocked_hosts: vec![],
            },
            process: ProcessPolicy {
                default_action: DefaultAction::Deny,
                allowed_commands: vec![],
                blocked_commands: vec![],
            },
        },
        signature: None,
    }
    .validate();

    let errors = result.expect_err("must fail");
    assert_eq!(errors.items[0].code, ValidationErrorCode::DuplicateValue);
    assert_eq!(errors.items[0].field, "policy.filesystem.readable_roots");
}

#[test]
fn reject_allow_block_conflict() {
    let result = compile_policy(&Policy {
        filesystem: FilesystemPolicy {
            readable_roots: vec![],
            writable_roots: vec![],
            blocked_roots: vec![],
        },
        network: NetworkPolicy {
            default_action: DefaultAction::Deny,
            allowed_hosts: vec!["api.openai.com".into()],
            blocked_hosts: vec!["api.openai.com".into()],
        },
        process: ProcessPolicy {
            default_action: DefaultAction::Deny,
            allowed_commands: vec![],
            blocked_commands: vec![],
        },
    });

    let errors = result.expect_err("must fail");
    assert_eq!(errors.items[0].code, ValidationErrorCode::ConflictingValue);
    assert_eq!(errors.items[0].field, "policy.network");
}

#[test]
fn reject_policy_when_capability_missing() {
    let policy = Policy {
        filesystem: FilesystemPolicy {
            readable_roots: vec!["src".into()],
            writable_roots: vec!["src".into()],
            blocked_roots: vec![],
        },
        network: NetworkPolicy {
            default_action: DefaultAction::Deny,
            allowed_hosts: vec!["api.openai.com".into()],
            blocked_hosts: vec![],
        },
        process: ProcessPolicy {
            default_action: DefaultAction::Deny,
            allowed_commands: vec![],
            blocked_commands: vec![],
        },
    };

    let snapshot = EngineCapabilitySnapshot {
        engine_name: "test-runtime".into(),
        engine_version: None,
        platform: "windows".into(),
        filesystem: FilesystemCapabilitySnapshot {
            enforcement: CapabilitySupport::Supported,
            observation: CapabilitySupport::Supported,
            temporary_file_coverage: CapabilitySupport::Limited,
            atomic_rename_coverage: CapabilitySupport::Limited,
        },
        network: NetworkCapabilitySnapshot {
            enforcement: CapabilitySupport::Unsupported,
            observation: CapabilitySupport::Unsupported,
            proxy_awareness: CapabilitySupport::Unsupported,
        },
        process: ProcessCapabilitySnapshot {
            enforcement: CapabilitySupport::Supported,
            observation: CapabilitySupport::Supported,
            termination: CapabilitySupport::Supported,
        },
    };

    let errors = validate_policy_against_capabilities(&policy, &snapshot).expect_err("must fail");
    assert_eq!(errors.items[0].code, ValidationErrorCode::UnsupportedCapability);
    assert_eq!(errors.items[0].field, "capabilities.network.enforcement");
}
