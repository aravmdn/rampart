use engine_greywall::{GreywallAdapter, GreywallVersion, RawEngineEventKind};
use policy_core::{AuditEventKind, IsolationMode};
use rampartd::{
    LaunchSessionRequest, LocalDataStore, RampartService, SelectedLaunchConfig, ServiceQueryApi,
};
use std::fs;
use std::path::PathBuf;

fn temp_dir(name: &str) -> PathBuf {
    let root = std::env::temp_dir()
        .join("rampart-tests")
        .join(format!("{}-{}", name, std::process::id()));
    if root.exists() {
        fs::remove_dir_all(&root).expect("temp dir should be removable");
    }
    fs::create_dir_all(&root).expect("temp dir should be created");
    root
}

fn adapter() -> GreywallAdapter {
    GreywallAdapter::from_binary_path(
        PathBuf::from("greywall"),
        GreywallVersion {
            major: 0,
            minor: 3,
            patch: 0,
        },
    )
    .expect("adapter should build")
}

#[test]
fn launcher_preferences_and_detected_projects_round_trip_through_store() {
    let data_dir = temp_dir("launcher-prefs");
    let store = LocalDataStore::open(&data_dir).expect("store should initialize");
    let service = RampartService::new(adapter(), store).expect("service should build");

    let launch_context = service.launch_context().expect("launch context should load");
    assert!(
        launch_context.projects.iter().any(|project| project.path.ends_with("rampart")),
        "detected projects should include the current repo root"
    );
    assert!(
        launch_context.profiles.iter().any(|profile| profile.id == "windows-safe"),
        "default presets should be available"
    );

    service
        .save_selected_launch_config(SelectedLaunchConfig {
            project_path: Some(r"C:\projects\rampart".into()),
            agent_id: Some("codex".into()),
            profile_id: Some("windows-safe".into()),
        })
        .expect("preferences should persist");

    let reloaded = RampartService::new(
        adapter(),
        LocalDataStore::open(&data_dir).expect("store should reopen"),
    )
    .expect("service should rebuild");
    let launch_context = reloaded.launch_context().expect("launch context should reload");

    assert_eq!(
        launch_context.selected.project_path.as_deref(),
        Some(r"C:\projects\rampart")
    );
    assert_eq!(launch_context.selected.agent_id.as_deref(), Some("codex"));
    assert_eq!(
        launch_context.selected.profile_id.as_deref(),
        Some("windows-safe")
    );
}

#[test]
fn session_history_persists_stop_and_blocked_events() {
    let data_dir = temp_dir("session-history");
    let store = LocalDataStore::open(&data_dir).expect("store should initialize");
    let mut service = RampartService::new(adapter(), store).expect("service should build");

    let session = service
        .launch_session(LaunchSessionRequest {
            project_dir: current_repo_root(),
            agent_id: "codex".into(),
            profile_id: "windows-safe".into(),
            isolation_mode: IsolationMode::WindowsNative,
        })
        .expect("launch should succeed");

    service
        .ingest_raw_event(
            &session.id,
            RawEngineEventKind::BlockRead,
            r"C:\Users\alice\.ssh\id_rsa",
        )
        .expect("blocked read should persist");
    service
        .stop_session(&session.id)
        .expect("stop should succeed");

    let history = service.session_history().expect("history should load");
    let record = history
        .iter()
        .find(|entry| entry.session.id == session.id)
        .expect("launched session should be recorded");

    assert!(
        record
            .events
            .iter()
            .any(|event| matches!(event.kind, AuditEventKind::FilesystemBlocked)),
        "history should include blocked events"
    );
    assert!(
        record.violations.iter().any(|violation| violation.target.ends_with("id_rsa")),
        "persisted violation should be queryable from history"
    );

    let reloaded = RampartService::new(
        adapter(),
        LocalDataStore::open(&data_dir).expect("store should reopen"),
    )
    .expect("service should rebuild");
    let history = reloaded.session_history().expect("history should reload");
    assert_eq!(history.len(), 1, "history should survive service restart");
}

fn current_repo_root() -> String {
    let mut cursor = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while let Some(parent) = cursor.parent() {
        if parent.join(".git").exists() {
            return parent.display().to_string();
        }
        cursor = parent.to_path_buf();
    }
    panic!("repo root should exist");
}
