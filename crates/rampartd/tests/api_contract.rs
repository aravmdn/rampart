use engine_greywall::{GreywallAdapter, GreywallVersion};
use policy_core::{
    DefaultAction, FilesystemPolicy, IsolationMode, NetworkPolicy, Policy, ProcessPolicy, Profile,
};
use rampartd::{DaemonApi, LaunchSessionRequest, RampartDaemon};
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

fn sample_daemon() -> RampartDaemon {
    let adapter = GreywallAdapter::from_binary_path(
        PathBuf::from("greywall"),
        GreywallVersion {
            major: 0,
            minor: 3,
            patch: 0,
        },
    )
    .expect("adapter must build");
    RampartDaemon::new(adapter, vec![sample_profile()])
}

#[test]
fn detects_capabilities_before_launch() {
    let daemon = sample_daemon();
    let snapshot = daemon.detect_capabilities().expect("capabilities should work");

    assert_eq!(snapshot.engine_name, "greywall");
    assert_eq!(snapshot.platform, "windows");
}

#[test]
fn launches_with_agent_project_and_profile_only() {
    let mut daemon = sample_daemon();
    let session = daemon
        .launch_session(LaunchSessionRequest {
            project_dir: r"C:\projects\rampart".into(),
            agent_id: "codex".into(),
            profile_id: "windows-safe".into(),
            isolation_mode: IsolationMode::WindowsNative,
        })
        .expect("launch should work");

    assert_eq!(session.project_path, r"C:\projects\rampart");
    assert_eq!(session.profile_id, "windows-safe");
}
