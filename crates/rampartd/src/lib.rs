use engine_greywall::{EnforcementEngine, GreywallAdapter, RawEngineEvent, RawEngineEventKind};
#[cfg(target_os = "windows")]
use engine_windows::{
    patch_project_low_integrity_label, set_process_low_integrity, EtwAuditProvider, WindowsJob,
    WfpNetworkGuard,
};
use policy_core::{
    compile_policy, validate_policy_against_capabilities, AgentTool, AuditEvent,
    AuditEventCategory, AuditEventKind, AuditOutcome, CapabilitySupport, EngineCapabilitySnapshot,
    Profile, Session, SessionStatus, ViolationEvent,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Agent adapters
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentAdapter {
    pub agent_id: String,
    pub command: String,
    pub default_args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub terminal_first: bool,
}

impl AgentAdapter {
    pub fn for_tool(tool: &AgentTool) -> Self {
        match tool {
            AgentTool::ClaudeCode => Self {
                agent_id: "claude-code".into(),
                command: "claude".into(),
                default_args: vec![],
                env: vec![],
                terminal_first: true,
            },
            AgentTool::Codex => Self {
                agent_id: "codex".into(),
                command: "codex".into(),
                default_args: vec![],
                env: vec![],
                terminal_first: true,
            },
            AgentTool::Cursor => Self {
                agent_id: "cursor".into(),
                command: "cursor-agent".into(),
                default_args: vec![],
                env: vec![],
                terminal_first: false,
            },
            AgentTool::Copilot => Self {
                agent_id: "copilot".into(),
                command: "github-copilot-cli".into(),
                default_args: vec![],
                env: vec![],
                terminal_first: false,
            },
            AgentTool::Aider => Self {
                agent_id: "aider".into(),
                command: "aider".into(),
                default_args: vec![],
                env: vec![],
                terminal_first: true,
            },
            AgentTool::Goose => Self {
                agent_id: "goose".into(),
                command: "goose".into(),
                default_args: vec![],
                env: vec![],
                terminal_first: true,
            },
            AgentTool::OpenCode => Self {
                agent_id: "opencode".into(),
                command: "opencode".into(),
                default_args: vec![],
                env: vec![],
                terminal_first: true,
            },
            AgentTool::GeminiCli => Self {
                agent_id: "gemini".into(),
                command: "gemini".into(),
                default_args: vec![],
                env: vec![],
                terminal_first: true,
            },
            AgentTool::Custom { id, .. } => Self {
                agent_id: id.clone(),
                command: id.clone(),
                default_args: vec![],
                env: vec![],
                terminal_first: true,
            },
        }
    }
}

// ---------------------------------------------------------------------------
// Preflight diagnostics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PreflightSeverity {
    Pass,
    Warning,
    Fail,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreflightDiagnostic {
    pub severity: PreflightSeverity,
    pub label: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreflightReport {
    pub ready: bool,
    pub diagnostics: Vec<PreflightDiagnostic>,
}

pub fn run_preflight(
    project_dir: &str,
    agent_tool: &AgentTool,
    profile: &Profile,
    capabilities: &EngineCapabilitySnapshot,
) -> PreflightReport {
    let mut diagnostics = Vec::new();

    // Check project directory exists
    let project_path = Path::new(project_dir);
    if project_path.exists() && project_path.is_dir() {
        diagnostics.push(PreflightDiagnostic {
            severity: PreflightSeverity::Pass,
            label: "Project directory".into(),
            detail: format!("{project_dir} exists and is a directory."),
        });
    } else {
        diagnostics.push(PreflightDiagnostic {
            severity: PreflightSeverity::Fail,
            label: "Project directory".into(),
            detail: format!("{project_dir} does not exist or is not a directory."),
        });
    }

    // Check agent binary is on PATH
    let adapter = AgentAdapter::for_tool(agent_tool);
    let binary_found = which_exists(&adapter.command);
    if binary_found {
        diagnostics.push(PreflightDiagnostic {
            severity: PreflightSeverity::Pass,
            label: "Agent binary".into(),
            detail: format!("{} found on PATH.", adapter.command),
        });
    } else {
        diagnostics.push(PreflightDiagnostic {
            severity: PreflightSeverity::Fail,
            label: "Agent binary".into(),
            detail: format!(
                "{} not found on PATH. Install the agent or check your PATH.",
                adapter.command
            ),
        });
    }

    // Terminal-first note
    if adapter.terminal_first {
        diagnostics.push(PreflightDiagnostic {
            severity: PreflightSeverity::Warning,
            label: "Terminal-first agent".into(),
            detail: format!(
                "{} is a terminal-first tool. Rampart will launch it but interaction happens in the agent\u{2019}s own terminal.",
                adapter.command
            ),
        });
    }

    // Capability warnings
    let cap_checks: Vec<(&str, CapabilitySupport)> = vec![
        ("Filesystem enforcement", capabilities.filesystem.enforcement),
        ("Network enforcement", capabilities.network.enforcement),
        ("Process enforcement", capabilities.process.enforcement),
    ];
    for (label, support) in cap_checks {
        match support {
            CapabilitySupport::Unsupported => {
                diagnostics.push(PreflightDiagnostic {
                    severity: PreflightSeverity::Warning,
                    label: label.into(),
                    detail: format!(
                        "{label} is unsupported on {} with {}. Policy rules for this domain will not be enforced.",
                        capabilities.platform, capabilities.engine_name
                    ),
                });
            }
            CapabilitySupport::Limited => {
                diagnostics.push(PreflightDiagnostic {
                    severity: PreflightSeverity::Warning,
                    label: label.into(),
                    detail: format!(
                        "{label} is limited on {} with {}. Some policy rules may not be fully enforced.",
                        capabilities.platform, capabilities.engine_name
                    ),
                });
            }
            CapabilitySupport::Supported => {
                diagnostics.push(PreflightDiagnostic {
                    severity: PreflightSeverity::Pass,
                    label: label.into(),
                    detail: format!("{label} supported."),
                });
            }
        }
    }

    // WSL2 stronger isolation mode availability (Windows only)
    if cfg!(target_os = "windows") {
        if detect_wsl2() {
            diagnostics.push(PreflightDiagnostic {
                severity: PreflightSeverity::Pass,
                label: "WSL2 isolation available".into(),
                detail: "WSL2 detected. Select \u{201c}WSL2\u{201d} isolation mode to run the agent inside a Linux VM with Landlock + seccomp enforcement instead of Windows-native controls.".into(),
            });
        } else {
            diagnostics.push(PreflightDiagnostic {
                severity: PreflightSeverity::Warning,
                label: "WSL2 not detected".into(),
                detail: "WSL2 is not installed or has no distributions. Stronger isolation mode is unavailable; Windows-native enforcement (Job Objects + WFP) will be used.".into(),
            });
        }
    }

    // Policy/capability compatibility
    if let Err(errors) = validate_policy_against_capabilities(&profile.policy, capabilities) {
        for item in &errors.items {
            diagnostics.push(PreflightDiagnostic {
                severity: PreflightSeverity::Warning,
                label: "Policy compatibility".into(),
                detail: format!("{}: {}", item.field, item.message),
            });
        }
    }

    let ready = !diagnostics
        .iter()
        .any(|d| matches!(d.severity, PreflightSeverity::Fail));

    PreflightReport { ready, diagnostics }
}

/// Returns true when WSL2 is available on this Windows host.
/// Runs `wsl --list --quiet`; exit-0 means at least one distribution is installed.
/// Always false on non-Windows.
fn detect_wsl2() -> bool {
    if !cfg!(target_os = "windows") {
        return false;
    }
    std::process::Command::new("wsl")
        .args(["--list", "--quiet"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn which_exists(command: &str) -> bool {
    // On Windows, check common extensions
    if cfg!(target_os = "windows") {
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in path_var.split(';') {
                let base = Path::new(dir).join(command);
                for ext in &["", ".exe", ".cmd", ".bat"] {
                    let candidate = base.with_extension(ext.trim_start_matches('.'));
                    if candidate.exists() {
                        return true;
                    }
                }
            }
        }
        false
    } else {
        if let Ok(path_var) = std::env::var("PATH") {
            for dir in path_var.split(':') {
                if Path::new(dir).join(command).exists() {
                    return true;
                }
            }
        }
        false
    }
}

// ---------------------------------------------------------------------------
// Flattened capability items for the UI
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlatCapabilityItem {
    pub key: String,
    pub status: String,
    pub detail: String,
}

pub fn flatten_capabilities(snapshot: &EngineCapabilitySnapshot) -> Vec<FlatCapabilityItem> {
    let support_str = |s: CapabilitySupport| -> &'static str {
        match s {
            CapabilitySupport::Supported => "supported",
            CapabilitySupport::Limited => "partial",
            CapabilitySupport::Unsupported => "unsupported",
        }
    };
    let detail = |label: &str, s: CapabilitySupport| -> String {
        match s {
            CapabilitySupport::Supported => format!("{label} fully supported on {}/{}.", snapshot.platform, snapshot.engine_name),
            CapabilitySupport::Limited => format!("{label} partially supported on {}/{}. Some gaps may exist.", snapshot.platform, snapshot.engine_name),
            CapabilitySupport::Unsupported => format!("{label} unsupported on {}/{}. Policy rules will not be enforced.", snapshot.platform, snapshot.engine_name),
        }
    };

    vec![
        FlatCapabilityItem {
            key: "filesystem_scope".into(),
            status: support_str(snapshot.filesystem.enforcement).into(),
            detail: detail("Filesystem scope enforcement", snapshot.filesystem.enforcement),
        },
        FlatCapabilityItem {
            key: "network_egress".into(),
            status: support_str(snapshot.network.enforcement).into(),
            detail: detail("Network egress enforcement", snapshot.network.enforcement),
        },
        FlatCapabilityItem {
            key: "process_execution".into(),
            status: support_str(snapshot.process.enforcement).into(),
            detail: detail("Process execution enforcement", snapshot.process.enforcement),
        },
        FlatCapabilityItem {
            key: "violation_streaming".into(),
            status: support_str(snapshot.filesystem.observation).into(),
            detail: detail("Violation event streaming", snapshot.filesystem.observation),
        },
        FlatCapabilityItem {
            key: "session_termination".into(),
            status: support_str(snapshot.process.termination).into(),
            detail: detail("Session termination", snapshot.process.termination),
        },
    ]
}

// ---------------------------------------------------------------------------
// Profile detail (flat view consumed by the UI)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileFilesystem {
    pub readable_roots: Vec<String>,
    pub writable_roots: Vec<String>,
    pub blocked_roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileNetwork {
    pub default_action: policy_core::DefaultAction,
    pub allowed_hosts: Vec<String>,
    pub blocked_hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProfileProcess {
    pub default_action: policy_core::DefaultAction,
    pub allowed_commands: Vec<String>,
    pub blocked_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDetail {
    pub id: String,
    pub display_name: String,
    pub detail: String,
    pub filesystem: ProfileFilesystem,
    pub network: ProfileNetwork,
    pub process: ProfileProcess,
}

impl ProfileDetail {
    fn from_profile(profile: &Profile) -> Self {
        Self {
            id: profile.id.clone(),
            display_name: profile.name.clone(),
            detail: profile.description.clone().unwrap_or_default(),
            filesystem: ProfileFilesystem {
                readable_roots: profile.policy.filesystem.readable_roots.clone(),
                writable_roots: profile.policy.filesystem.writable_roots.clone(),
                blocked_roots: profile.policy.filesystem.blocked_roots.clone(),
            },
            network: ProfileNetwork {
                default_action: profile.policy.network.default_action,
                allowed_hosts: profile.policy.network.allowed_hosts.clone(),
                blocked_hosts: profile.policy.network.blocked_hosts.clone(),
            },
            process: ProfileProcess {
                default_action: profile.policy.process.default_action,
                allowed_commands: profile.policy.process.allowed_commands.clone(),
                blocked_commands: profile.policy.process.blocked_commands.clone(),
            },
        }
    }

    fn into_profile(self) -> Profile {
        use policy_core::{FilesystemPolicy, NetworkPolicy, Policy, ProcessPolicy};
        Profile {
            id: self.id,
            name: self.display_name,
            description: if self.detail.is_empty() { None } else { Some(self.detail) },
            extends: None,
            policy: Policy {
                filesystem: FilesystemPolicy {
                    readable_roots: self.filesystem.readable_roots,
                    writable_roots: self.filesystem.writable_roots,
                    blocked_roots: self.filesystem.blocked_roots,
                },
                network: NetworkPolicy {
                    default_action: self.network.default_action,
                    allowed_hosts: self.network.allowed_hosts,
                    blocked_hosts: self.network.blocked_hosts,
                },
                process: ProcessPolicy {
                    default_action: self.process.default_action,
                    allowed_commands: self.process.allowed_commands,
                    blocked_commands: self.process.blocked_commands,
                },
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaunchSessionRequest {
    pub project_dir: String,
    pub agent_tool: AgentTool,
    pub profile_id: String,
    #[serde(default)]
    pub isolation_mode: policy_core::IsolationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SessionEventRecord {
    Audit(AuditEvent),
    Violation(ViolationEvent),
}

pub trait DaemonApi {
    fn detect_capabilities(&self) -> Result<EngineCapabilitySnapshot, DaemonError>;
    fn list_profiles(&self) -> &[Profile];
    fn launch_session(&mut self, request: LaunchSessionRequest) -> Result<Session, DaemonError>;
    fn stop_session(&mut self, session_id: &str) -> Result<Session, DaemonError>;
    fn stream_session_events(&self, session_id: &str) -> Result<Vec<SessionEventRecord>, DaemonError>;
}

pub struct RampartDaemon<E = GreywallAdapter> {
    engine: E,
    runner: LocalProcessRunner,
    profiles: Vec<Profile>,
    sessions: HashMap<String, Session>,
    events: HashMap<String, VecDeque<SessionEventRecord>>,
    capability_snapshots: HashMap<String, EngineCapabilitySnapshot>,
    active_processes: HashMap<String, ManagedProcess>,
    next_session_id: u64,
    #[cfg(target_os = "windows")]
    etw: Option<EtwAuditProvider>,
}

impl<E> RampartDaemon<E>
where
    E: EnforcementEngine,
{
    pub fn new(engine: E, profiles: Vec<Profile>) -> Self {
        Self::new_with_runner(engine, LocalProcessRunner::default(), profiles)
    }

    pub fn new_with_runner(engine: E, runner: LocalProcessRunner, profiles: Vec<Profile>) -> Self {
        Self {
            engine,
            runner,
            profiles,
            sessions: HashMap::new(),
            events: HashMap::new(),
            capability_snapshots: HashMap::new(),
            active_processes: HashMap::new(),
            next_session_id: 1,
            #[cfg(target_os = "windows")]
            etw: EtwAuditProvider::register()
                .map_err(|e| eprintln!("rampartd: ETW registration failed: {e}"))
                .ok(),
        }
    }

    #[allow(unused_variables)]
    fn emit_audit_etw(&self, event: &policy_core::AuditEvent) {
        #[cfg(target_os = "windows")]
        if let Some(etw) = &self.etw {
            etw.write_audit_event(
                &event.session_id,
                &format!("{:?}", event.kind),
                &format!("{:?}", event.outcome),
                &event.message,
            );
        }
    }

    pub fn ingest_raw_event(
        &mut self,
        session_id: &str,
        kind: RawEngineEventKind,
        target: impl Into<String>,
    ) -> Result<(), DaemonError> {
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| DaemonError::UnknownSession(session_id.into()))?;
        let next_sequence = self
            .events
            .get(session_id)
            .map(|queue| queue.len() as u64 + 1)
            .unwrap_or(1);
        let normalized = self.engine.normalize_event(RawEngineEvent {
            session_id: session.id.clone(),
            sequence: next_sequence,
            occurred_at_ms: session.started_at_ms + next_sequence,
            kind,
            target: target.into(),
        });

        self.emit_audit_etw(&normalized.audit);
        let queue = self.events.entry(session_id.into()).or_default();
        queue.push_back(SessionEventRecord::Audit(normalized.audit));
        if let Some(violation) = normalized.violation {
            queue.push_back(SessionEventRecord::Violation(violation));
        }
        Ok(())
    }

    pub fn session(&self, session_id: &str) -> Result<Session, DaemonError> {
        self.sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| DaemonError::UnknownSession(session_id.into()))
    }

    pub fn session_history_record(&self, session_id: &str) -> Result<SessionHistoryRecord, DaemonError> {
        let session = self.session(session_id)?;
        let events = self.stream_session_events(session_id)?;
        let mut audit = Vec::new();
        let mut violations = Vec::new();

        for entry in events {
            match entry {
                SessionEventRecord::Audit(item) => audit.push(item),
                SessionEventRecord::Violation(item) => violations.push(item),
            }
        }

        let capability_snapshot = self.capability_snapshots.get(session_id).cloned();

        Ok(SessionHistoryRecord {
            session,
            capability_snapshot,
            events: audit,
            violations,
        })
    }
}

impl<E> DaemonApi for RampartDaemon<E>
where
    E: EnforcementEngine,
{
    fn detect_capabilities(&self) -> Result<EngineCapabilitySnapshot, DaemonError> {
        Ok(self.engine.capability_snapshot())
    }

    fn list_profiles(&self) -> &[Profile] {
        &self.profiles
    }

    fn launch_session(&mut self, request: LaunchSessionRequest) -> Result<Session, DaemonError> {
        let profile = self
            .profiles
            .iter()
            .find(|profile| profile.id == request.profile_id)
            .ok_or_else(|| DaemonError::UnknownProfile(request.profile_id.clone()))?;

        profile.validate().map_err(DaemonError::InvalidProfile)?;
        let compiled_policy = compile_policy(&profile.policy).map_err(DaemonError::InvalidPolicy)?;

        let session_id = format!("session-{}", self.next_session_id);
        self.next_session_id += 1;

        let started_at_ms = now_ms();
        let snapshot = self.engine.capability_snapshot();
        self.capability_snapshots.insert(session_id.clone(), snapshot);

        let session = Session {
            id: session_id.clone(),
            project_path: request.project_dir,
            agent_tool: request.agent_tool,
            profile_id: profile.id.clone(),
            compiled_policy,
            status: SessionStatus::Running,
            started_at_ms,
            ended_at_ms: None,
            isolation_mode: request.isolation_mode,
        };
        let process_result = self.runner.spawn_session(&session);
        let (persisted_session, launch_audit) = match process_result {
            Ok(process) => {
                self.active_processes.insert(session_id.clone(), process.clone());
                (
                    session,
                    AuditEvent {
                        session_id: session_id.clone(),
                        sequence: 1,
                        occurred_at_ms: started_at_ms,
                        kind: AuditEventKind::SessionLaunched,
                        category: AuditEventCategory::SessionLifecycle,
                        outcome: AuditOutcome::Info,
                        message: format!(
                            "Session launched through Rampart daemon. pid={}",
                            process.pid.unwrap_or_default()
                        ),
                        violation: None,
                    },
                )
            }
            Err(error) => {
                let mut failed = session;
                failed.status = SessionStatus::Failed;
                failed.ended_at_ms = Some(started_at_ms);
                (
                    failed,
                    AuditEvent {
                        session_id: session_id.clone(),
                        sequence: 1,
                        occurred_at_ms: started_at_ms,
                        kind: AuditEventKind::AlertRaised,
                        category: AuditEventCategory::SystemAlert,
                        outcome: AuditOutcome::Error,
                        message: format!("Session launch failed: {error}"),
                        violation: None,
                    },
                )
            }
        };
        self.sessions
            .insert(session_id.clone(), persisted_session.clone());
        self.emit_audit_etw(&launch_audit);
        self.events.insert(
            session_id.clone(),
            VecDeque::from([SessionEventRecord::Audit(launch_audit)]),
        );
        Ok(persisted_session)
    }

    fn stop_session(&mut self, session_id: &str) -> Result<Session, DaemonError> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| DaemonError::UnknownSession(session_id.into()))?;
        if matches!(session.status, SessionStatus::Running) {
            self.runner.stop_session(session_id)?;
            self.active_processes.remove(session_id);
        }
        session.status = SessionStatus::Terminated;
        let ended_at_ms = now_ms().max(session.started_at_ms);
        session.ended_at_ms = Some(ended_at_ms);

        let next_sequence = self
            .events
            .get(session_id)
            .map(|queue| queue.len() as u64 + 1)
            .unwrap_or(2);

        let end_event = AuditEvent {
            session_id: session_id.into(),
            sequence: next_sequence,
            occurred_at_ms: ended_at_ms,
            kind: AuditEventKind::SessionEnded,
            category: AuditEventCategory::SessionLifecycle,
            outcome: AuditOutcome::Info,
            message: "Session stopped by daemon.".into(),
            violation: None,
        };
        self.emit_audit_etw(&end_event);
        self.events
            .entry(session_id.into())
            .or_default()
            .push_back(SessionEventRecord::Audit(end_event));

        Ok(session.clone())
    }

    fn stream_session_events(&self, session_id: &str) -> Result<Vec<SessionEventRecord>, DaemonError> {
        let queue = self
            .events
            .get(session_id)
            .ok_or_else(|| DaemonError::UnknownSession(session_id.into()))?;
        Ok(queue.iter().cloned().collect())
    }
}

#[derive(Debug, Error)]
pub enum DaemonError {
    #[error("profile validation failed")]
    InvalidProfile(#[source] policy_core::ValidationErrors),
    #[error("policy compilation failed")]
    InvalidPolicy(#[source] policy_core::ValidationErrors),
    #[error("unknown profile '{0}'")]
    UnknownProfile(String),
    #[error("unknown session '{0}'")]
    UnknownSession(String),
    #[error(transparent)]
    Runner(#[from] ProcessRunnerError),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManagedProcess {
    pub pid: Option<u32>,
    pub command: String,
}

pub struct LocalProcessRunner {
    children: HashMap<String, Child>,
    #[cfg(target_os = "windows")]
    jobs: HashMap<String, WindowsJob>,
    #[cfg(target_os = "windows")]
    wfp_guards: HashMap<String, WfpNetworkGuard>,
}

impl Default for LocalProcessRunner {
    fn default() -> Self {
        Self {
            children: HashMap::new(),
            #[cfg(target_os = "windows")]
            jobs: HashMap::new(),
            #[cfg(target_os = "windows")]
            wfp_guards: HashMap::new(),
        }
    }
}

impl LocalProcessRunner {
    fn spawn_session(&mut self, session: &Session) -> Result<ManagedProcess, ProcessRunnerError> {
        let adapter = AgentAdapter::for_tool(&session.agent_tool);
        let wsl2 = matches!(session.isolation_mode, policy_core::IsolationMode::Wsl2);

        let mut cmd = if wsl2 {
            // Run the agent inside WSL2: `wsl --cd <linux_path> -- <command> [args]`
            let mut c = Command::new("wsl");
            c.arg("--cd")
                .arg(win_path_to_wsl(&session.project_path))
                .arg("--")
                .arg(&adapter.command);
            c
        } else {
            let mut c = Command::new(&adapter.command);
            c.current_dir(&session.project_path);
            c
        };
        cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        for arg in &adapter.default_args {
            cmd.arg(arg);
        }
        for (key, value) in &adapter.env {
            cmd.env(key, value);
        }

        // Windows-native enforcement (Job Objects + Low Integrity + WFP + SACL).
        // Skipped in WSL2 mode: the agent runs inside a Linux VM where the host
        // Win32 security primitives do not apply.
        #[cfg(target_os = "windows")]
        if !wsl2 {
            if let Err(error) = patch_project_low_integrity_label(Path::new(&session.project_path)) {
                eprintln!(
                    "rampartd: SACL patch failed for '{}': {error}",
                    session.project_path
                );
            }
        }

        let child = cmd.spawn().map_err(|source| ProcessRunnerError::Spawn {
            command: adapter.command.clone(),
            source,
        })?;
        let pid = child.id();

        #[cfg(target_os = "windows")]
        if !wsl2 {
            match WindowsJob::assign(pid) {
                Ok(job) => {
                    self.jobs.insert(session.id.clone(), job);
                }
                Err(error) => {
                    eprintln!(
                        "rampartd: Job Object assignment failed for session '{}' (pid {pid}): {error}",
                        session.id
                    );
                }
            }

            if let Err(error) = set_process_low_integrity(pid) {
                eprintln!(
                    "rampartd: Low integrity not applied for session '{}' (pid {pid}): {error}",
                    session.id
                );
            }

            match WfpNetworkGuard::open(pid, &adapter.command) {
                Ok(guard) => {
                    self.wfp_guards.insert(session.id.clone(), guard);
                }
                Err(error) => {
                    eprintln!(
                        "rampartd: WFP filter failed for session '{}' (pid {pid}): {error}",
                        session.id
                    );
                }
            }
        }

        self.children.insert(session.id.clone(), child);
        Ok(ManagedProcess {
            pid: Some(pid),
            command: adapter.command,
        })
    }

    fn stop_session(&mut self, session_id: &str) -> Result<(), ProcessRunnerError> {
        // Drop the job handle first so KILL_ON_JOB_CLOSE fires before we call
        // child.kill(). On non-Windows the map removals are no-op compile-outs.
        #[cfg(target_os = "windows")]
        drop(self.jobs.remove(session_id));
        #[cfg(target_os = "windows")]
        drop(self.wfp_guards.remove(session_id));

        let mut child = self
            .children
            .remove(session_id)
            .ok_or_else(|| ProcessRunnerError::UnknownSession(session_id.into()))?;
        child.kill().map_err(|source| ProcessRunnerError::Stop {
            session_id: session_id.into(),
            source,
        })
    }
}

#[derive(Debug, Error)]
pub enum ProcessRunnerError {
    #[error("session command '{command}' could not be launched")]
    Spawn {
        command: String,
        #[source]
        source: std::io::Error,
    },
    #[error("session '{0}' has no active process")]
    UnknownSession(String),
    #[error("session '{session_id}' could not be stopped")]
    Stop {
        session_id: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentCatalogEntry {
    pub id: String,
    pub label: String,
    pub detail: String,
    pub terminal_first: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectSummary {
    pub id: String,
    pub label: String,
    pub path: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileSummary {
    pub id: String,
    pub display_name: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SelectedLaunchConfig {
    pub project_path: Option<String>,
    pub agent_id: Option<String>,
    pub profile_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaunchContext {
    pub projects: Vec<ProjectSummary>,
    pub agents: Vec<AgentCatalogEntry>,
    pub profiles: Vec<ProfileSummary>,
    pub selected: SelectedLaunchConfig,
    pub capabilities: EngineCapabilitySnapshot,
    pub capability_items: Vec<FlatCapabilityItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionHistoryRecord {
    pub session: Session,
    /// Capability snapshot captured at session launch time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capability_snapshot: Option<EngineCapabilitySnapshot>,
    pub events: Vec<AuditEvent>,
    pub violations: Vec<ViolationEvent>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
struct PersistedState {
    selected: SelectedLaunchConfig,
    history: Vec<SessionHistoryRecord>,
    #[serde(default)]
    custom_profiles: Vec<ProfileDetail>,
}

#[derive(Debug, Clone)]
pub struct LocalDataStore {
    root: PathBuf,
    state_path: PathBuf,
}

impl LocalDataStore {
    pub fn open(root: impl AsRef<Path>) -> Result<Self, StoreError> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root).map_err(|source| StoreError::CreateDirectory {
            path: root.clone(),
            source,
        })?;

        let state_path = root.join("local-state.json");
        if !state_path.exists() {
            let initial = serde_json::to_string_pretty(&PersistedState::default())
                .map_err(StoreError::Serialize)?;
            fs::write(&state_path, initial).map_err(|source| StoreError::WriteState {
                path: state_path.clone(),
                source,
            })?;
        }

        Ok(Self { root, state_path })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn load_state(&self) -> Result<PersistedState, StoreError> {
        let raw = fs::read_to_string(&self.state_path).map_err(|source| StoreError::ReadState {
            path: self.state_path.clone(),
            source,
        })?;
        serde_json::from_str(&raw).map_err(StoreError::Deserialize)
    }

    fn save_state(&self, state: &PersistedState) -> Result<(), StoreError> {
        let serialized = serde_json::to_string_pretty(state).map_err(StoreError::Serialize)?;
        fs::write(&self.state_path, serialized).map_err(|source| StoreError::WriteState {
            path: self.state_path.clone(),
            source,
        })
    }
}

pub trait ServiceQueryApi {
    fn launch_context(&self) -> Result<LaunchContext, ServiceError>;
    fn session_history(&self) -> Result<Vec<SessionHistoryRecord>, ServiceError>;
    fn save_selected_launch_config(
        &self,
        selected: SelectedLaunchConfig,
    ) -> Result<(), ServiceError>;
    fn preflight_check(
        &self,
        project_dir: &str,
        agent_id: &str,
        profile_id: &str,
    ) -> Result<PreflightReport, ServiceError>;
    fn load_profile(&self, profile_id: &str) -> Result<ProfileDetail, ServiceError>;
    fn save_profile(&self, detail: ProfileDetail) -> Result<(), ServiceError>;
}

pub struct RampartService<E = GreywallAdapter> {
    daemon: RampartDaemon<E>,
    store: LocalDataStore,
    selected: SelectedLaunchConfig,
}

impl<E> RampartService<E>
where
    E: EnforcementEngine,
{
    pub fn new(engine: E, store: LocalDataStore) -> Result<Self, ServiceError> {
        let state = store.load_state()?;
        let daemon = RampartDaemon::new(engine, default_profiles());

        Ok(Self {
            daemon,
            store,
            selected: state.selected,
        })
    }

    pub fn launch_session(&mut self, request: LaunchSessionRequest) -> Result<Session, ServiceError> {
        let session = self.daemon.launch_session(request)?;
        self.persist_history_record(&session.id)?;
        Ok(session)
    }

    pub fn stop_session(&mut self, session_id: &str) -> Result<Session, ServiceError> {
        let session = self.daemon.stop_session(session_id)?;
        self.persist_history_record(session_id)?;
        Ok(session)
    }

    pub fn ingest_raw_event(
        &mut self,
        session_id: &str,
        kind: RawEngineEventKind,
        target: impl Into<String>,
    ) -> Result<(), ServiceError> {
        self.daemon.ingest_raw_event(session_id, kind, target)?;
        self.persist_history_record(session_id)
    }

    fn persist_history_record(&self, session_id: &str) -> Result<(), ServiceError> {
        let mut state = self.store.load_state()?;
        let record = self.daemon.session_history_record(session_id)?;
        upsert_history_record(&mut state.history, record);
        self.store.save_state(&state)?;
        Ok(())
    }
}

impl<E> DaemonApi for RampartService<E>
where
    E: EnforcementEngine,
{
    fn detect_capabilities(&self) -> Result<EngineCapabilitySnapshot, DaemonError> {
        self.daemon.detect_capabilities()
    }

    fn list_profiles(&self) -> &[Profile] {
        self.daemon.list_profiles()
    }

    fn launch_session(&mut self, request: LaunchSessionRequest) -> Result<Session, DaemonError> {
        self.daemon.launch_session(request)
    }

    fn stop_session(&mut self, session_id: &str) -> Result<Session, DaemonError> {
        self.daemon.stop_session(session_id)
    }

    fn stream_session_events(&self, session_id: &str) -> Result<Vec<SessionEventRecord>, DaemonError> {
        self.daemon.stream_session_events(session_id)
    }
}

impl<E> ServiceQueryApi for RampartService<E>
where
    E: EnforcementEngine,
{
    fn launch_context(&self) -> Result<LaunchContext, ServiceError> {
        let capabilities = self.daemon.detect_capabilities()?;
        let capability_items = flatten_capabilities(&capabilities);
        let project_root = detect_repo_root()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| r"C:\projects\rampart".into());
        let profiles =
            profiles_for_agent(self.selected.agent_id.as_deref(), &project_root);
        Ok(LaunchContext {
            projects: detect_projects()?,
            agents: default_agents(),
            profiles: profiles
                .iter()
                .map(|profile| ProfileSummary {
                    id: profile.id.clone(),
                    display_name: profile.name.clone(),
                    detail: profile
                        .description
                        .clone()
                        .unwrap_or_else(|| "Profile preset".into()),
                })
                .collect(),
            selected: self.selected.clone(),
            capabilities,
            capability_items,
        })
    }

    fn session_history(&self) -> Result<Vec<SessionHistoryRecord>, ServiceError> {
        Ok(self.store.load_state()?.history)
    }

    fn save_selected_launch_config(
        &self,
        selected: SelectedLaunchConfig,
    ) -> Result<(), ServiceError> {
        let mut state = self.store.load_state()?;
        state.selected = selected;
        self.store.save_state(&state)?;
        Ok(())
    }

    fn preflight_check(
        &self,
        project_dir: &str,
        agent_id: &str,
        profile_id: &str,
    ) -> Result<PreflightReport, ServiceError> {
        let agent_tool = agent_tool_from_id(agent_id);
        // Search agent-specific presets first, then the full merged list on the daemon.
        let project_root = detect_repo_root()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| r"C:\projects\rampart".into());
        let agent_profiles = profiles_for_agent(Some(agent_id), &project_root);
        let profile = agent_profiles
            .iter()
            .find(|p| p.id == profile_id)
            .cloned()
            .or_else(|| {
                self.daemon
                    .list_profiles()
                    .iter()
                    .find(|p| p.id == profile_id)
                    .cloned()
            })
            .ok_or_else(|| ServiceError::Daemon(DaemonError::UnknownProfile(profile_id.into())))?;
        let capabilities = self.daemon.detect_capabilities()?;
        Ok(run_preflight(project_dir, &agent_tool, &profile, &capabilities))
    }

    fn load_profile(&self, profile_id: &str) -> Result<ProfileDetail, ServiceError> {
        let state = self.store.load_state()?;
        // Custom profiles take precedence over presets.
        if let Some(custom) = state.custom_profiles.iter().find(|p| p.id == profile_id) {
            return Ok(custom.clone());
        }
        // Fall back to presets.
        let project_root = detect_repo_root()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| r"C:\projects\rampart".into());
        for tool in &[
            policy_core::AgentTool::ClaudeCode,
            policy_core::AgentTool::Codex,
            policy_core::AgentTool::Aider,
        ] {
            for preset in policy_core::agent_profile_presets(tool, &project_root) {
                if preset.id == profile_id {
                    return Ok(ProfileDetail::from_profile(&preset));
                }
            }
        }
        for preset in policy_core::desktop_profile_presets(&project_root) {
            if preset.id == profile_id {
                return Ok(ProfileDetail::from_profile(&preset));
            }
        }
        Err(ServiceError::Daemon(DaemonError::UnknownProfile(profile_id.into())))
    }

    fn save_profile(&self, detail: ProfileDetail) -> Result<(), ServiceError> {
        let mut state = self.store.load_state()?;
        if let Some(existing) = state.custom_profiles.iter_mut().find(|p| p.id == detail.id) {
            *existing = detail;
        } else {
            state.custom_profiles.push(detail);
        }
        self.store.save_state(&state)?;
        Ok(())
    }
}

fn upsert_history_record(history: &mut Vec<SessionHistoryRecord>, record: SessionHistoryRecord) {
    if let Some(existing) = history.iter_mut().find(|entry| entry.session.id == record.session.id) {
        *existing = record;
        return;
    }
    history.push(record);
    history.sort_by(|left, right| right.session.started_at_ms.cmp(&left.session.started_at_ms));
}

fn detect_projects() -> Result<Vec<ProjectSummary>, ServiceError> {
    let repo_root = detect_repo_root()?;
    let label = repo_root
        .file_name()
        .and_then(|segment| segment.to_str())
        .unwrap_or("Project")
        .to_string();

    Ok(vec![ProjectSummary {
        id: slugify(&label),
        label,
        path: repo_root.display().to_string(),
        source: "detected-current-workspace".into(),
    }])
}

fn detect_repo_root() -> Result<PathBuf, ServiceError> {
    let mut cursor = std::env::current_dir().map_err(ServiceError::CurrentDirectory)?;
    loop {
        if cursor.join(".git").exists() {
            return Ok(cursor);
        }
        if !cursor.pop() {
            return Err(ServiceError::ProjectDetection(
                "unable to find repository root from current working directory".into(),
            ));
        }
    }
}

fn default_agents() -> Vec<AgentCatalogEntry> {
    vec![
        AgentCatalogEntry {
            id: "codex".into(),
            label: "Codex".into(),
            detail: "OpenAI coding agent launched through Rampart session controls.".into(),
            terminal_first: true,
        },
        AgentCatalogEntry {
            id: "claude-code".into(),
            label: "Claude Code".into(),
            detail: "Terminal-first Anthropic agent with profile-driven launch.".into(),
            terminal_first: true,
        },
    ]
}

fn agent_tool_from_id(id: &str) -> AgentTool {
    match id {
        "codex" => AgentTool::Codex,
        "claude-code" => AgentTool::ClaudeCode,
        "cursor" => AgentTool::Cursor,
        "copilot" => AgentTool::Copilot,
        "aider" => AgentTool::Aider,
        "goose" => AgentTool::Goose,
        "opencode" => AgentTool::OpenCode,
        "gemini" => AgentTool::GeminiCli,
        other => AgentTool::Custom {
            id: other.into(),
            display_name: None,
        },
    }
}

fn default_profiles() -> Vec<Profile> {
    let root = detect_repo_root()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|_| r"C:\projects\rampart".into());
    // Merge all agent-specific presets so launch_session can find any profile by ID.
    let mut all: Vec<Profile> = vec![];
    for tool in &[
        AgentTool::ClaudeCode,
        AgentTool::Codex,
        AgentTool::Aider,
    ] {
        all.extend(policy_core::agent_profile_presets(tool, &root));
    }
    all.extend(policy_core::desktop_profile_presets(&root));
    all
}

/// Profiles to show in the launcher for a given agent selection.
fn profiles_for_agent(agent_id: Option<&str>, project_root: &str) -> Vec<Profile> {
    match agent_id {
        Some(id) => {
            let tool = agent_tool_from_id(id);
            let presets = policy_core::agent_profile_presets(&tool, project_root);
            // agent_profile_presets falls back to generic when there are no agent-specific
            // presets, so we always get a non-empty list.
            presets
        }
        None => policy_core::desktop_profile_presets(project_root),
    }
}

fn slugify(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

/// Convert a Windows absolute path to its WSL2 mount path.
/// `C:\foo\bar` → `/mnt/c/foo/bar`. Falls back to forward-slash substitution.
fn win_path_to_wsl(path: &str) -> String {
    let chars: Vec<char> = path.chars().collect();
    if chars.len() >= 3
        && chars[0].is_ascii_alphabetic()
        && chars[1] == ':'
        && (chars[2] == '\\' || chars[2] == '/')
    {
        let drive = chars[0].to_ascii_lowercase();
        let rest: String = path[3..].chars().map(|c| if c == '\\' { '/' } else { c }).collect();
        return format!("/mnt/{drive}/{rest}");
    }
    path.replace('\\', "/")
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error(transparent)]
    Daemon(#[from] DaemonError),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error("current directory lookup failed")]
    CurrentDirectory(#[source] std::io::Error),
    #[error("project detection failed: {0}")]
    ProjectDetection(String),
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("failed to create directory '{path}'")]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read state file '{path}'")]
    ReadState {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to write state file '{path}'")]
    WriteState {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to serialize local state")]
    Serialize(#[source] serde_json::Error),
    #[error("failed to deserialize local state")]
    Deserialize(#[source] serde_json::Error),
}
