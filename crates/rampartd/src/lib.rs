use engine_greywall::{GreywallAdapter, RawEngineEvent, RawEngineEventKind};
use policy_core::{
    AgentTool, AuditEvent, AuditEventKind, EngineCapabilitySnapshot, Platform, Profile, Session,
    SessionState, ViolationEvent,
};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSessionRequest {
    pub project_root: PathBuf,
    pub agent_tool: AgentTool,
    pub profile_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

pub struct RampartDaemon {
    adapter: GreywallAdapter,
    profiles: Vec<Profile>,
    sessions: HashMap<String, Session>,
    events: HashMap<String, VecDeque<SessionEventRecord>>,
    next_session_id: u64,
}

impl RampartDaemon {
    pub fn new(adapter: GreywallAdapter, profiles: Vec<Profile>) -> Self {
        Self {
            adapter,
            profiles,
            sessions: HashMap::new(),
            events: HashMap::new(),
            next_session_id: 1,
        }
    }

    pub fn ingest_raw_event(
        &mut self,
        session_id: &str,
        kind: RawEngineEventKind,
        target: impl Into<String>,
    ) -> Result<(), DaemonError> {
        if !self.sessions.contains_key(session_id) {
            return Err(DaemonError::UnknownSession(session_id.into()));
        }

        let batch = self.adapter.normalize_event(RawEngineEvent {
            session_id: session_id.into(),
            kind,
            target: target.into(),
        });
        let queue = self.events.entry(session_id.into()).or_default();
        queue.push_back(SessionEventRecord::Audit(batch.audit));
        if let Some(violation) = batch.violation {
            queue.push_back(SessionEventRecord::Violation(violation));
        }
        Ok(())
    }
}

impl DaemonApi for RampartDaemon {
    fn detect_capabilities(&self) -> Result<EngineCapabilitySnapshot, DaemonError> {
        Ok(self.adapter.capability_snapshot(Platform::Windows))
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

        let session_id = format!("session-{}", self.next_session_id);
        self.next_session_id += 1;

        let session = Session {
            id: session_id.clone(),
            project_root: request.project_root,
            agent_tool: request.agent_tool,
            profile_id: request.profile_id,
            state: SessionState::Active,
        };

        self.sessions.insert(session_id.clone(), session.clone());
        self.events.insert(
            session_id.clone(),
            VecDeque::from([SessionEventRecord::Audit(AuditEvent {
                session_id: session_id.clone(),
                kind: AuditEventKind::LaunchSucceeded,
                message: "Session launched through daemon stub.".into(),
            })]),
        );

        Ok(session)
    }

    fn stop_session(&mut self, session_id: &str) -> Result<Session, DaemonError> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| DaemonError::UnknownSession(session_id.into()))?;
        session.state = SessionState::Stopped;
        self.events
            .entry(session_id.into())
            .or_default()
            .push_back(SessionEventRecord::Audit(AuditEvent {
                session_id: session_id.into(),
                kind: AuditEventKind::SessionStopped,
                message: "Session stopped by daemon.".into(),
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
    #[error("profile validation failed: {0}")]
    InvalidProfile(#[source] policy_core::ValidationError),
    #[error("unknown profile '{0}'")]
    UnknownProfile(String),
    #[error("unknown session '{0}'")]
    UnknownSession(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use engine_greywall::GreywallVersion;
    use policy_core::{FilesystemPolicy, NetworkMode, NetworkPolicy, Policy, ProcessPolicy};

    fn sample_profile() -> Profile {
        Profile {
            id: "windows-safe".into(),
            display_name: "Windows Safe".into(),
            agent_tool: AgentTool::Codex,
            policy: Policy {
                project_root: PathBuf::from(r"C:\projects\rampart"),
                filesystem: FilesystemPolicy {
                    read_roots: vec![PathBuf::from(r"C:\projects\rampart")],
                    write_roots: vec![PathBuf::from(r"C:\projects\rampart\src")],
                    allow_temp_writes: false,
                },
                network: NetworkPolicy {
                    mode: NetworkMode::DenyAll,
                    allowed_domains: Vec::new(),
                },
                process: ProcessPolicy {
                    allow_child_processes: false,
                    allowed_commands: Vec::new(),
                },
            },
        }
    }

    fn sample_daemon() -> RampartDaemon {
        let adapter = GreywallAdapter::from_binary_path(
            PathBuf::from("greywall"),
            GreywallVersion { major: 0, minor: 3, patch: 0 },
        )
        .expect("adapter should build");
        RampartDaemon::new(adapter, vec![sample_profile()])
    }

    #[test]
    fn lists_capabilities_before_launch() {
        let daemon = sample_daemon();

        let snapshot = daemon.detect_capabilities().expect("capabilities should work");

        assert_eq!(snapshot.platform, Platform::Windows);
        assert!(!snapshot.capabilities.is_empty());
    }

    #[test]
    fn launches_and_streams_normalized_block_event() {
        let mut daemon = sample_daemon();
        let session = daemon
            .launch_session(LaunchSessionRequest {
                project_root: PathBuf::from(r"C:\projects\rampart"),
                agent_tool: AgentTool::Codex,
                profile_id: "windows-safe".into(),
            })
            .expect("launch should work");

        daemon
            .ingest_raw_event(
                &session.id,
                RawEngineEventKind::BlockRead,
                r"C:\Users\dev\.ssh\config",
            )
            .expect("ingest should work");

        let events = daemon
            .stream_session_events(&session.id)
            .expect("stream should work");

        assert!(events.iter().any(|event| matches!(event, SessionEventRecord::Violation(_))));
    }
}
