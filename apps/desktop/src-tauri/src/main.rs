#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
use engine_windows::WindowsEnforcer;
#[cfg(not(target_os = "windows"))]
use engine_greywall::GreywallAdapter;

#[cfg(target_os = "windows")]
type ActiveEngine = WindowsEnforcer;
#[cfg(not(target_os = "windows"))]
type ActiveEngine = GreywallAdapter;

use rampartd::{
    DaemonApi, LaunchSessionRequest, LocalDataStore, OrgPolicy, PreflightReport, ProfileDetail,
    RampartService, SelectedLaunchConfig, ServiceQueryApi, SessionEventRecord, SessionHistoryRecord,
    SyncConfig, SyncStatus,
};
use serde::Serialize;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::State;

struct DesktopDaemonState {
    service: Mutex<RampartService<ActiveEngine>>,
}

#[derive(Serialize)]
struct SessionEventBatch {
    audit: Vec<policy_core::AuditEvent>,
    violations: Vec<policy_core::ViolationEvent>,
}

#[tauri::command]
fn load_launch_context(
    state: State<'_, DesktopDaemonState>,
) -> Result<rampartd::LaunchContext, String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .launch_context()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_selected_launch_config(
    state: State<'_, DesktopDaemonState>,
    selected: SelectedLaunchConfig,
) -> Result<(), String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .save_selected_launch_config(selected)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn launch_session(
    state: State<'_, DesktopDaemonState>,
    request: LaunchSessionRequest,
) -> Result<policy_core::Session, String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .launch_session(request)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn stop_session(
    state: State<'_, DesktopDaemonState>,
    session_id: String,
) -> Result<policy_core::Session, String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .stop_session(&session_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn stream_session_events(
    state: State<'_, DesktopDaemonState>,
    session_id: String,
) -> Result<SessionEventBatch, String> {
    let events = state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .stream_session_events(&session_id)
        .map_err(|error| error.to_string())?;

    let mut audit = Vec::new();
    let mut violations = Vec::new();
    for event in events {
        match event {
            SessionEventRecord::Audit(item) => audit.push(item),
            SessionEventRecord::Violation(item) => violations.push(item),
        }
    }

    Ok(SessionEventBatch { audit, violations })
}

#[tauri::command]
fn list_session_history(
    state: State<'_, DesktopDaemonState>,
) -> Result<Vec<SessionHistoryRecord>, String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .session_history()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn load_profile(
    state: State<'_, DesktopDaemonState>,
    profile_id: String,
) -> Result<ProfileDetail, String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .load_profile(&profile_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn save_profile(
    state: State<'_, DesktopDaemonState>,
    profile: ProfileDetail,
) -> Result<(), String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .save_profile(profile)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn sign_profile(
    state: State<'_, DesktopDaemonState>,
    profile_id: String,
    signing_key_b64: String,
) -> Result<(), String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .sign_profile(&profile_id, &signing_key_b64)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn load_remote_profile(
    state: State<'_, DesktopDaemonState>,
    url: String,
) -> Result<ProfileDetail, String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .load_remote_profile(&url)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn configure_sync(
    state: State<'_, DesktopDaemonState>,
    config: SyncConfig,
) -> Result<(), String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .configure_sync(config)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_sync_status(state: State<'_, DesktopDaemonState>) -> Result<SyncStatus, String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .get_sync_status()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn sync_audit_events(state: State<'_, DesktopDaemonState>) -> Result<SyncStatus, String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .sync_audit_events()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn configure_org_policy_url(
    state: State<'_, DesktopDaemonState>,
    url: Option<String>,
) -> Result<(), String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .configure_org_policy_url(url)
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn fetch_org_policy(state: State<'_, DesktopDaemonState>) -> Result<Option<OrgPolicy>, String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .fetch_org_policy()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn current_org_policy(state: State<'_, DesktopDaemonState>) -> Result<Option<OrgPolicy>, String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .current_org_policy()
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn preflight_check(
    state: State<'_, DesktopDaemonState>,
    project_dir: String,
    agent_id: String,
    profile_id: String,
) -> Result<PreflightReport, String> {
    state
        .service
        .lock()
        .map_err(|_| "daemon state lock poisoned".to_string())?
        .preflight_check(&project_dir, &agent_id, &profile_id)
        .map_err(|error| error.to_string())
}

fn build_service() -> Result<RampartService<ActiveEngine>, String> {
    #[cfg(target_os = "windows")]
    let engine = WindowsEnforcer::new();
    #[cfg(not(target_os = "windows"))]
    let engine = GreywallAdapter::discover().map_err(|error| error.to_string())?;

    let root = std::env::current_dir()
        .map_err(|error| error.to_string())?
        .join(".rampart");
    let store = LocalDataStore::open(root).map_err(|error| error.to_string())?;
    RampartService::new(engine, store).map_err(|error| error.to_string())
}

fn main() {
    let service = build_service().expect("desktop daemon service should initialize");
    tauri::Builder::default()
        .manage(DesktopDaemonState {
            service: Mutex::new(service),
        })
        .invoke_handler(tauri::generate_handler![
            load_launch_context,
            save_selected_launch_config,
            launch_session,
            stop_session,
            stream_session_events,
            list_session_history,
            preflight_check,
            load_profile,
            save_profile,
            sign_profile,
            load_remote_profile,
            configure_sync,
            get_sync_status,
            sync_audit_events,
            configure_org_policy_url,
            fetch_org_policy,
            current_org_policy,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Rampart desktop shell");
}
