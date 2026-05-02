use engine_greywall::GreywallAdapter;
use policy_core::{DefaultAction, FilesystemPolicy, NetworkPolicy, Policy, ProcessPolicy, Profile};
use rampartd::{DaemonApi, LaunchSessionRequest, RampartDaemon};

fn main() {
    let adapter = GreywallAdapter::discover().expect("static stub config should stay valid");
    let profile = Profile {
        id: "windows-safe".into(),
        name: "Windows Safe".into(),
        description: Some("Project scoped default-deny profile.".into()),
        extends: None,
        signature: None,
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
    };
    let mut daemon = RampartDaemon::new(adapter, vec![profile]);
    let snapshot = daemon.detect_capabilities().expect("capabilities should work");
    println!(
        "capability snapshot ready: engine={}, platform={}",
        snapshot.engine_name, snapshot.platform
    );

    let session = daemon
        .launch_session(LaunchSessionRequest {
            project_dir: r"C:\projects\rampart".into(),
            agent_id: "codex".into(),
            profile_id: "windows-safe".into(),
            isolation_mode: policy_core::IsolationMode::default(),
        })
        .expect("launch should work");
    println!("session launched: {}", session.id);
}
