use engine_greywall::{EnforcementEngine, GreywallAdapter, RawEngineEvent, RawEngineEventKind};
use policy_core::{
    compile_policy, AgentTool, AuditEvent, AuditEventKind, AuditOutcome, EngineCapabilitySnapshot,
    Profile, Session, SessionStatus, ViolationEvent,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaunchSessionRequest {
    pub project_dir: String,
    pub agent_tool: AgentTool,
    pub profile_id: String,
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
    active_processes: HashMap<String, ManagedProcess>,
    next_session_id: u64,
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
            active_processes: HashMap::new(),
            next_session_id: 1,
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

        Ok(SessionHistoryRecord {
            session,
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

        let session = Session {
            id: session_id.clone(),
            project_path: request.project_dir,
            agent_tool: request.agent_tool,
            profile_id: profile.id.clone(),
            compiled_policy,
            status: SessionStatus::Running,
            started_at_ms,
            ended_at_ms: None,
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
                        outcome: AuditOutcome::Error,
                        message: format!("Session launch failed: {error}"),
                        violation: None,
                    },
                )
            }
        };
        self.sessions
            .insert(session_id.clone(), persisted_session.clone());
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

        self.events
            .entry(session_id.into())
            .or_default()
            .push_back(SessionEventRecord::Audit(AuditEvent {
                session_id: session_id.into(),
                sequence: next_sequence,
                occurred_at_ms: ended_at_ms,
                kind: AuditEventKind::SessionEnded,
                outcome: AuditOutcome::Info,
                message: "Session stopped by daemon.".into(),
                violation: None,
            }));

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

#[derive(Default)]
pub struct LocalProcessRunner {
    children: HashMap<String, Child>,
}

impl LocalProcessRunner {
    fn spawn_session(&mut self, session: &Session) -> Result<ManagedProcess, ProcessRunnerError> {
        let command = agent_command_name(&session.agent_tool).to_string();
        let child = Command::new(&command)
            .current_dir(&session.project_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|source| ProcessRunnerError::Spawn {
                command: command.clone(),
                source,
            })?;
        let pid = child.id();
        self.children.insert(session.id.clone(), child);
        Ok(ManagedProcess {
            pid: Some(pid),
            command,
        })
    }

    fn stop_session(&mut self, session_id: &str) -> Result<(), ProcessRunnerError> {
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

fn agent_command_name(agent_tool: &AgentTool) -> &str {
    match agent_tool {
        AgentTool::Codex => "codex",
        AgentTool::ClaudeCode => "claude",
        AgentTool::Cursor => "cursor-agent",
        AgentTool::Copilot => "github-copilot-cli",
        AgentTool::Aider => "aider",
        AgentTool::Goose => "goose",
        AgentTool::OpenCode => "opencode",
        AgentTool::GeminiCli => "gemini",
        AgentTool::Custom { id, .. } => id.as_str(),
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionHistoryRecord {
    pub session: Session,
    pub events: Vec<AuditEvent>,
    pub violations: Vec<ViolationEvent>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
struct PersistedState {
    selected: SelectedLaunchConfig,
    history: Vec<SessionHistoryRecord>,
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
        Ok(LaunchContext {
            projects: detect_projects()?,
            agents: default_agents(),
            profiles: self
                .daemon
                .list_profiles()
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
            capabilities: self.daemon.detect_capabilities()?,
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
        },
        AgentCatalogEntry {
            id: "claude-code".into(),
            label: "Claude Code".into(),
            detail: "Terminal-first Anthropic agent with profile-driven launch.".into(),
        },
    ]
}

fn default_profiles() -> Vec<Profile> {
    policy_core::desktop_profile_presets(
        &detect_repo_root()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| r"C:\projects\rampart".into()),
    )
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
