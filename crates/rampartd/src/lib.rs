use engine_greywall::{EnforcementEngine, GreywallAdapter, RawEngineEvent, RawEngineEventKind};
use policy_core::{
    compile_policy, AgentTool, AuditEvent, AuditEventKind, AuditOutcome, EngineCapabilitySnapshot,
    Profile, Session, SessionStatus, ViolationEvent,
};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSessionRequest {
    pub project_dir: String,
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

pub struct RampartDaemon<E = GreywallAdapter> {
    engine: E,
    profiles: Vec<Profile>,
    sessions: HashMap<String, Session>,
    events: HashMap<String, VecDeque<SessionEventRecord>>,
    next_session_id: u64,
}

impl<E> RampartDaemon<E>
where
    E: EnforcementEngine,
{
    pub fn new(engine: E, profiles: Vec<Profile>) -> Self {
        Self {
            engine,
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

        let session = Session {
            id: session_id.clone(),
            project_path: request.project_dir,
            agent_tool: request.agent_tool,
            profile_id: profile.id.clone(),
            compiled_policy,
            status: SessionStatus::Running,
            started_at_ms: 1_000,
            ended_at_ms: None,
        };
        self.sessions.insert(session_id.clone(), session.clone());
        self.events.insert(
            session_id.clone(),
            VecDeque::from([SessionEventRecord::Audit(AuditEvent {
                session_id: session_id.clone(),
                sequence: 1,
                occurred_at_ms: session.started_at_ms,
                kind: AuditEventKind::SessionLaunched,
                outcome: AuditOutcome::Info,
                message: "Session launched through daemon stub.".into(),
                violation: None,
            })]),
        );
        Ok(session)
    }

    fn stop_session(&mut self, session_id: &str) -> Result<Session, DaemonError> {
        let session = self
            .sessions
            .get_mut(session_id)
            .ok_or_else(|| DaemonError::UnknownSession(session_id.into()))?;
        session.status = SessionStatus::Terminated;
        session.ended_at_ms = Some(session.started_at_ms + 50);

        self.events
            .entry(session_id.into())
            .or_default()
            .push_back(SessionEventRecord::Audit(AuditEvent {
                session_id: session_id.into(),
                sequence: 99,
                occurred_at_ms: session.started_at_ms + 50,
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
}
