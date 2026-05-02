use engine_greywall::{GreywallAdapter, GreywallVersion, RawEngineEventKind};
use policy_core::{
    AuditEventKind, DefaultAction, FilesystemPolicy, IsolationMode, NetworkPolicy, Policy,
    ProcessPolicy, Profile,
};
use rampartd::{DaemonApi, LaunchSessionRequest, RampartDaemon, SessionEventRecord};
use std::path::PathBuf;

fn sample_profile() -> Profile {
    Profile {
        id: "windows-safe".into(),
        name: "Windows Safe".into(),
        description: Some("Project scoped default-deny profile.".into()),
        extends: None,
        policy: Policy {
            filesystem: FilesystemPolicy {
                readable_roots: vec![r"C:\projects\rampart".into()],
                writable_roots: vec![r"C:\projects\rampart\src".into()],
                blocked_roots: vec![r"C:\Users".into()],
            },
            network: NetworkPolicy {
                default_action: DefaultAction::Deny,
                allowed_hosts: Vec::new(),
                blocked_hosts: Vec::new(),
            },
            process: ProcessPolicy {
                default_action: DefaultAction::Deny,
                allowed_commands: vec!["git".into()],
                blocked_commands: vec!["powershell".into()],
            },
        },
    }
}

#[test]
fn blocked_action_loop_records_launch_then_normalized_blocked_read() {
    let adapter = GreywallAdapter::from_binary_path(
        PathBuf::from("greywall"),
        GreywallVersion {
            major: 0,
            minor: 3,
            patch: 0,
        },
    )
    .expect("adapter should build");
    let mut daemon = RampartDaemon::new(adapter, vec![sample_profile()]);

    let session = daemon
        .launch_session(LaunchSessionRequest {
            project_dir: r"C:\projects\rampart".into(),
            agent_id: "codex".into(),
            profile_id: "windows-safe".into(),
            isolation_mode: IsolationMode::WindowsNative,
        })
        .expect("launch should work");

    daemon
        .ingest_raw_event(
            &session.id,
            RawEngineEventKind::BlockRead,
            r"C:\Users\alice\.ssh\id_rsa",
        )
        .expect("raw event should normalize");

    let events = daemon
        .stream_session_events(&session.id)
        .expect("event stream should load");

    assert_eq!(events.len(), 3, "launch audit + blocked audit + violation");

    let blocked_audit = events
        .iter()
        .find_map(|event| match event {
            SessionEventRecord::Audit(audit) if audit.kind == AuditEventKind::ViolationRecorded => {
                Some(audit)
            }
            _ => None,
        })
        .expect("blocked audit should exist");
    assert!(blocked_audit.message.contains(r"C:\Users\alice\.ssh\id_rsa"));

    let violation = events
        .iter()
        .find_map(|event| match event {
            SessionEventRecord::Violation(violation) => Some(violation),
            _ => None,
        })
        .expect("violation should exist");

    assert_eq!(violation.rule_id, "fs.read.project-root-only");
    assert_eq!(
        violation.rule_label,
        "Read access limited to selected project root."
    );
    assert_eq!(
        violation.reason,
        "Policy denied access outside allowed project roots."
    );
}
