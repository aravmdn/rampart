use engine_greywall::{EnforcementEngine, GreywallAdapter, RawEngineEvent, RawEngineEventKind};
use engine_windows::BlockedNetworkEvent;
#[cfg(target_os = "windows")]
use engine_windows::{
    patch_project_low_integrity_label, set_process_low_integrity, EtwAuditProvider, WindowsJob,
    WfpEventMonitor, WfpNetworkGuard,
};
use policy_core::{
    compile_policy, validate_policy_against_capabilities, AgentTool, AuditEvent,
    AuditEventCategory, AuditEventKind, AuditOutcome, CapabilitySupport, EngineCapabilitySnapshot,
    OrgPolicy, Profile, Session, SessionStatus, ViolationEvent,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

// Re-export OrgPolicy and IsolationMode for use by binaries
pub use policy_core::OrgPolicy;
pub use policy_core::IsolationMode;

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
    /// True when this diagnostic was added or promoted to a more severe level
    /// because the org policy floor made the effective profile stricter than
    /// the local profile alone would have been.
    #[serde(default)]
    pub from_org_policy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PreflightReport {
    pub ready: bool,
    pub diagnostics: Vec<PreflightDiagnostic>,
}

/// Return the effective profile to use for a given session context.
///
/// If `org` is `Some` and `org_policy_applies` returns true for (agent_type, project_path),
/// the merged profile from `resolve_effective_policy` is returned. Otherwise the local profile
/// is cloned unchanged.
pub fn effective_profile_for(
    local: &Profile,
    org: Option<&OrgPolicy>,
    agent_type: &str,
    project_path: &str,
) -> Profile {
    match org {
        Some(org_policy) if policy_core::org_policy_applies(org_policy, agent_type, project_path) => {
            policy_core::resolve_effective_policy(local, org_policy)
        }
        _ => local.clone(),
    }
}

pub fn run_preflight(
    project_dir: &str,
    agent_tool: &AgentTool,
    profile: &Profile,
    capabilities: &EngineCapabilitySnapshot,
    org: Option<&OrgPolicy>,
) -> PreflightReport {
    // Determine the agent type string for scope matching.
    let agent_type = AgentAdapter::for_tool(agent_tool).agent_id;

    // Compute the effective (merged) profile. If no org policy applies, this is
    // identical to the local profile.
    let effective = effective_profile_for(profile, org, &agent_type, project_dir);

    // Run preflight against local profile and against the effective profile.
    // Any diagnostic present in the merged run but absent in the local run (matched
    // by label+detail), or whose severity escalated, is annotated from_org_policy=true.
    //
    // Strategy: run the inner logic twice (local, then merged). Compare by (label, detail)
    // key; anything new or severity-escalated in the merged set gets the marker.
    let adapter = AgentAdapter::for_tool(agent_tool);
    let command = adapter.command;
    let terminal_first = adapter.terminal_first;
    let local_diags = run_preflight_inner(project_dir, &command, terminal_first, profile, capabilities);
    let merged_diags = run_preflight_inner(project_dir, &command, terminal_first, &effective, capabilities);

    // Build a lookup of local diagnostics by (label, detail) for O(n) comparison.
    let local_set: std::collections::HashSet<(String, String)> = local_diags
        .iter()
        .map(|d| (d.label.clone(), d.detail.clone()))
        .collect();

    // Build a lookup of local severity by label for escalation detection.
    let local_severity: std::collections::HashMap<String, &PreflightSeverity> = local_diags
        .iter()
        .map(|d| (d.label.clone(), &d.severity))
        .collect();

    let diagnostics: Vec<PreflightDiagnostic> = merged_diags
        .into_iter()
        .map(|mut d| {
            let key = (d.label.clone(), d.detail.clone());
            let is_new = !local_set.contains(&key);
            let is_escalated = local_severity
                .get(&d.label)
                .map(|local_sev| severity_rank(&d.severity) > severity_rank(local_sev))
                .unwrap_or(false);
            if is_new || is_escalated {
                d.from_org_policy = true;
            }
            d
        })
        .collect();

    // Inject an informational diagnostic when org policy is active and applies.
    let mut final_diagnostics = diagnostics;
    if org.map(|o| policy_core::org_policy_applies(o, &agent_type, project_dir)).unwrap_or(false) {
        // Prepend an informational notice (not from_org_policy itself — it's a meta note).
        final_diagnostics.insert(0, PreflightDiagnostic {
            severity: PreflightSeverity::Warning,
            label: "Org policy floor active".into(),
            detail: "An org policy applies to this session. Some profile settings may be stricter than your local profile; diagnostics marked with org policy origin reflect those restrictions.".into(),
            from_org_policy: false,
        });
    }

    let ready = !final_diagnostics
        .iter()
        .any(|d| matches!(d.severity, PreflightSeverity::Fail));

    PreflightReport { ready, diagnostics: final_diagnostics }
}

/// Numeric rank for severity comparison: higher = more severe.
fn severity_rank(s: &PreflightSeverity) -> u8 {
    match s {
        PreflightSeverity::Pass => 0,
        PreflightSeverity::Warning => 1,
        PreflightSeverity::Fail => 2,
    }
}

/// Inner preflight logic that runs against a concrete profile (no org merging).
/// `command` is the binary name to check on PATH (e.g. `"claude"`, not the adapter id).
/// Returns diagnostics without `from_org_policy` annotations (all false).
fn run_preflight_inner(
    project_dir: &str,
    command: &str,
    terminal_first: bool,
    profile: &Profile,
    capabilities: &EngineCapabilitySnapshot,
) -> Vec<PreflightDiagnostic> {
    let mut diagnostics = Vec::new();

    // Check project directory exists
    let project_path = Path::new(project_dir);
    if project_path.exists() && project_path.is_dir() {
        diagnostics.push(PreflightDiagnostic {
            severity: PreflightSeverity::Pass,
            label: "Project directory".into(),
            detail: format!("{project_dir} exists and is a directory."),
            from_org_policy: false,
        });
    } else {
        diagnostics.push(PreflightDiagnostic {
            severity: PreflightSeverity::Fail,
            label: "Project directory".into(),
            detail: format!("{project_dir} does not exist or is not a directory."),
            from_org_policy: false,
        });
    }

    // Check agent binary is on PATH.
    // The binary check is profile-independent; same result for both local and
    // merged runs. We include it in both so label-based comparison is consistent.
    let binary_found = which_exists(command);
    if binary_found {
        diagnostics.push(PreflightDiagnostic {
            severity: PreflightSeverity::Pass,
            label: "Agent binary".into(),
            detail: format!("{command} found on PATH."),
            from_org_policy: false,
        });
    } else {
        diagnostics.push(PreflightDiagnostic {
            severity: PreflightSeverity::Fail,
            label: "Agent binary".into(),
            detail: format!(
                "{command} not found on PATH. Install the agent or check your PATH.",
            ),
            from_org_policy: false,
        });
    }

    // Terminal-first note (agent-dependent, profile-independent).
    if terminal_first {
        diagnostics.push(PreflightDiagnostic {
            severity: PreflightSeverity::Warning,
            label: "Terminal-first agent".into(),
            detail: format!(
                "{command} is a terminal-first tool. Rampart will launch it but interaction happens in the agent\u{2019}s own terminal.",
            ),
            from_org_policy: false,
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
                    from_org_policy: false,
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
                    from_org_policy: false,
                });
            }
            CapabilitySupport::Supported => {
                diagnostics.push(PreflightDiagnostic {
                    severity: PreflightSeverity::Pass,
                    label: label.into(),
                    detail: format!("{label} supported."),
                    from_org_policy: false,
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
                from_org_policy: false,
            });
        } else {
            diagnostics.push(PreflightDiagnostic {
                severity: PreflightSeverity::Warning,
                label: "WSL2 not detected".into(),
                detail: "WSL2 is not installed or has no distributions. Stronger isolation mode is unavailable; Windows-native enforcement (Job Objects + WFP) will be used.".into(),
                from_org_policy: false,
            });
        }
    }

    // Signature validity: an Invalid signature hard-blocks launch
    let sig_status = policy_core::verify_signature(profile);
    if matches!(sig_status, policy_core::SignatureStatus::Invalid) {
        diagnostics.push(PreflightDiagnostic {
            severity: PreflightSeverity::Fail,
            label: "Profile signature".into(),
            detail: "Profile signature is invalid — the profile may have been tampered with. Fix the signature or replace the profile before launching.".into(),
            from_org_policy: false,
        });
    }

    // Policy/capability compatibility
    if let Err(errors) = validate_policy_against_capabilities(&profile.policy, capabilities) {
        for item in &errors.items {
            diagnostics.push(PreflightDiagnostic {
                severity: PreflightSeverity::Warning,
                label: "Policy compatibility".into(),
                detail: format!("{}: {}", item.field, item.message),
                from_org_policy: false,
            });
        }
    }

    diagnostics
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
    /// Stored signature block; absent for unsigned profiles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<policy_core::ProfileSignature>,
    /// Computed on every load; never persisted. Defaults to Unsigned.
    #[serde(default)]
    pub signature_status: policy_core::SignatureStatus,
}

impl ProfileDetail {
    fn from_profile(profile: &Profile) -> Self {
        let signature_status = policy_core::verify_signature(profile);
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
            signature: profile.signature.clone(),
            signature_status,
        }
    }

    fn into_profile(self) -> Profile {
        use policy_core::{FilesystemPolicy, NetworkPolicy, Policy, ProcessPolicy};
        Profile {
            id: self.id,
            name: self.display_name,
            description: if self.detail.is_empty() { None } else { Some(self.detail) },
            extends: None,
            signature: self.signature,
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
    #[serde(rename = "projectPath")]
    pub project_dir: String,
    #[serde(rename = "agentId")]
    pub agent_id: String,
    #[serde(rename = "profileId")]
    pub profile_id: String,
    #[serde(rename = "isolationMode", default)]
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
            agent_tool: agent_tool_from_id(&request.agent_id),
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
        {
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
        }

        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| DaemonError::UnknownSession(session_id.into()))?;
        let ended_at_ms = session.ended_at_ms.unwrap_or_else(now_ms);

        // Drain any buffered WFP events and persist them into the permanent queue
        // before computing next_sequence, so SessionEnded gets a correct sequence
        // number and the events appear in session history.
        {
            let wfp_events = self.runner.drain_wfp_events(session_id);
            if !wfp_events.is_empty() {
                let base_seq = self
                    .events
                    .get(session_id)
                    .map(|q| q.len() as u64)
                    .unwrap_or(0);
                let queue = self.events.entry(session_id.into()).or_default();
                for (i, ev) in wfp_events.into_iter().enumerate() {
                    queue.push_back(SessionEventRecord::Audit(AuditEvent {
                        session_id: session_id.into(),
                        sequence: base_seq + 1 + i as u64,
                        occurred_at_ms: ev.occurred_at_ms,
                        kind: AuditEventKind::NetworkBlocked,
                        category: AuditEventCategory::PolicyEnforcement,
                        outcome: AuditOutcome::Blocked,
                        message: format!(
                            "WFP blocked outbound connection to port {}",
                            ev.remote_port
                        ),
                        violation: None,
                    }));
                }
            }
            // Drop the monitor (unsubscribes from WFP). Must happen after drain
            // so no events are lost between drain and unsubscribe.
            #[cfg(target_os = "windows")]
            self.runner.wfp_monitors.remove(session_id);
        }

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
        let result = session.clone();
        self.emit_audit_etw(&end_event);
        self.events
            .entry(session_id.into())
            .or_default()
            .push_back(SessionEventRecord::Audit(end_event));

        Ok(result)
    }

    fn stream_session_events(&self, session_id: &str) -> Result<Vec<SessionEventRecord>, DaemonError> {
        let queue = self
            .events
            .get(session_id)
            .ok_or_else(|| DaemonError::UnknownSession(session_id.into()))?;
        let mut result: Vec<SessionEventRecord> = queue.iter().cloned().collect();

        // Append WFP blocked-connection events from the live monitor. These are
        // peeked (not drained) so they reappear on every poll until persisted at
        // stop_session. Sequence numbers are assigned relative to the current list.
        let base_seq = result.len() as u64 + 1;
        for (i, ev) in self.runner.peek_wfp_events(session_id).into_iter().enumerate() {
            result.push(SessionEventRecord::Audit(AuditEvent {
                session_id: session_id.into(),
                sequence: base_seq + i as u64,
                occurred_at_ms: ev.occurred_at_ms,
                kind: AuditEventKind::NetworkBlocked,
                category: AuditEventCategory::PolicyEnforcement,
                outcome: AuditOutcome::Blocked,
                message: format!(
                    "WFP blocked outbound connection to port {}",
                    ev.remote_port
                ),
                violation: None,
            }));
        }

        Ok(result)
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
    #[cfg(target_os = "windows")]
    wfp_monitors: HashMap<String, WfpEventMonitor>,
}

impl Default for LocalProcessRunner {
    fn default() -> Self {
        Self {
            children: HashMap::new(),
            #[cfg(target_os = "windows")]
            jobs: HashMap::new(),
            #[cfg(target_os = "windows")]
            wfp_guards: HashMap::new(),
            #[cfg(target_os = "windows")]
            wfp_monitors: HashMap::new(),
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
                    let nt_path = guard.nt_path.clone();
                    self.wfp_guards.insert(session.id.clone(), guard);
                    // Start the event monitor on the same NT path so blocked
                    // connections surface in the session console.
                    match WfpEventMonitor::start(&nt_path) {
                        Ok(monitor) => {
                            self.wfp_monitors.insert(session.id.clone(), monitor);
                        }
                        Err(error) => {
                            eprintln!(
                                "rampartd: WFP event monitor failed for session '{}': {error}",
                                session.id
                            );
                        }
                    }
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

    /// Snapshot (clone) all buffered WFP blocked-connection events without consuming
    /// them. Events remain in the buffer and reappear on the next call.
    #[allow(unused_variables, unreachable_code)]
    fn peek_wfp_events(&self, session_id: &str) -> Vec<BlockedNetworkEvent> {
        #[cfg(target_os = "windows")]
        return self
            .wfp_monitors
            .get(session_id)
            .map(|m| m.peek())
            .unwrap_or_default();
        Vec::new()
    }

    /// Drain all buffered WFP blocked-connection events, emptying the buffer.
    /// Called at session stop to persist events into the permanent audit queue.
    #[allow(unused_variables, unreachable_code)]
    fn drain_wfp_events(&self, session_id: &str) -> Vec<BlockedNetworkEvent> {
        #[cfg(target_os = "windows")]
        return self
            .wfp_monitors
            .get(session_id)
            .map(|m| m.drain())
            .unwrap_or_default();
        Vec::new()
    }

    /// Returns true if the managed child process for the session is still running.
    /// Returns false if the process has exited or the session is not found.
    pub fn is_session_running(&mut self, session_id: &str) -> bool {
        match self.children.get_mut(session_id) {
            Some(child) => child.try_wait().map(|status| status.is_none()).unwrap_or(false),
            None => false,
        }
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
    #[serde(default)]
    pub signature_status: policy_core::SignatureStatus,
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
    #[serde(default)]
    sync_config: Option<SyncConfig>,
    #[serde(default)]
    audit_queue: Vec<AuditQueueEntry>,
    #[serde(default)]
    last_sync_at_ms: Option<u64>,
    #[serde(default)]
    last_sync_error: Option<String>,
    #[serde(default)]
    org_policy_url: Option<String>,
    #[serde(default)]
    cached_org_policy: Option<OrgPolicy>,
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

    pub fn profile_cache_dir(&self) -> PathBuf {
        self.root.join("profile-cache")
    }

    pub fn org_policy_cache_dir(&self) -> PathBuf {
        self.root.join("org-policy-cache")
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

// ---------------------------------------------------------------------------
// Sync config + status
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncConfig {
    pub endpoint_url: String,
    pub token: String,
    pub strip_paths: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub configured: bool,
    pub queue_depth: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_sync_at_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct AuditQueueEntry {
    session_id: String,
    event: policy_core::AuditEvent,
    queued_at_ms: u64,
    sent: bool,
}

/// Drain unsent audit queue entries to the configured sync endpoint.
/// Fire-and-forget: errors are recorded in persisted state but not propagated.
fn drain_audit_queue(store: &LocalDataStore) {
    let mut state = match store.load_state() {
        Ok(s) => s,
        Err(_) => return,
    };
    let config = match state.sync_config.clone() {
        Some(c) => c,
        None => return,
    };
    let pending_indices: Vec<usize> = state
        .audit_queue
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.sent)
        .map(|(i, _)| i)
        .take(500)
        .collect();
    if pending_indices.is_empty() {
        return;
    }
    let events: Vec<serde_json::Value> = pending_indices
        .iter()
        .map(|&i| {
            let entry = &state.audit_queue[i];
            let mut evt = serde_json::to_value(&entry.event).unwrap_or_default();
            if config.strip_paths {
                if let Some(obj) = evt.as_object_mut() {
                    obj.insert("message".into(), serde_json::Value::String("<redacted>".into()));
                }
            }
            evt
        })
        .collect();
    let payload = serde_json::json!({ "events": events });
    let send_result = (|| -> Result<(), String> {
        let client = reqwest::blocking::Client::new();
        let resp = client
            .post(&config.endpoint_url)
            .header("Authorization", format!("Bearer {}", config.token))
            .json(&payload)
            .send()
            .map_err(|e| e.to_string())?;
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(format!("HTTP {}", resp.status()))
        }
    })();
    let now = now_ms();
    match send_result {
        Ok(()) => {
            for &i in &pending_indices {
                state.audit_queue[i].sent = true;
            }
            state.last_sync_at_ms = Some(now);
            state.last_sync_error = None;
        }
        Err(ref err) => {
            state.last_sync_error = Some(err.clone());
        }
    }
    let _ = store.save_state(&state);
}

/// Generic fetch from HTTPS URL with disk cache and network fallback.
/// Caches raw JSON bytes under `cache_dir` and deserializes to type `T`.
/// On network error, falls back to cached copy if it exists.
fn fetch_remote_cached<T: serde::de::DeserializeOwned>(
    url: &str,
    cache_dir: &Path,
) -> Result<T, ServiceError> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let cache_file = cache_dir.join(format!("{:016x}.json", hasher.finish()));

    let fetch_result = (|| -> Result<Vec<u8>, String> {
        let resp = reqwest::blocking::get(url).map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }
        resp.bytes().map(|b| b.to_vec()).map_err(|e| e.to_string())
    })();

    match fetch_result {
        Ok(body) => {
            let item: T = serde_json::from_slice(&body)
                .map_err(|e| ServiceError::RemoteFetch(format!("invalid JSON: {e}")))?;
            let _ = fs::create_dir_all(cache_dir);
            let _ = fs::write(&cache_file, &body);
            Ok(item)
        }
        Err(network_err) => {
            if cache_file.exists() {
                let cached = fs::read_to_string(&cache_file)
                    .map_err(|e| ServiceError::RemoteFetch(e.to_string()))?;
                serde_json::from_str(&cached)
                    .map_err(|e| ServiceError::RemoteFetch(e.to_string()))
            } else {
                Err(ServiceError::RemoteFetch(network_err))
            }
        }
    }
}

/// Fetch a `Profile` from an HTTPS URL, caching the raw JSON bytes under `cache_dir`.
/// On network failure, falls back to the cached copy if it exists.
fn fetch_remote_profile(url: &str, cache_dir: &Path) -> Result<Profile, ServiceError> {
    fetch_remote_cached(url, cache_dir)
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
    /// Sign a stored custom profile with the given ed25519 private key seed (base64-encoded).
    /// Rejects tampered profiles and unknown profile IDs.
    fn sign_profile(&self, profile_id: &str, signing_key_b64: &str) -> Result<(), ServiceError>;
    /// Fetch a signed profile from an HTTPS URL, cache to disk, and return its detail.
    fn load_remote_profile(&self, url: &str) -> Result<ProfileDetail, ServiceError>;
    /// Persist audit sync configuration.
    fn configure_sync(&self, config: SyncConfig) -> Result<(), ServiceError>;
    /// Return current sync queue status without sending anything.
    fn get_sync_status(&self) -> Result<SyncStatus, ServiceError>;
    /// Drain the unsent audit queue by POSTing to the configured endpoint.
    /// Returns updated status. No-op if sync is not configured.
    fn sync_audit_events(&self) -> Result<SyncStatus, ServiceError>;
    /// Set or clear the org policy URL in persisted state.
    fn configure_org_policy_url(&mut self, url: Option<String>) -> Result<(), ServiceError>;
    /// Fetch org policy from configured URL, cache to disk, and return it.
    /// If URL is unset, returns Ok(None). On network error, falls back to disk cache.
    fn fetch_org_policy(&mut self) -> Result<Option<OrgPolicy>, ServiceError>;
    /// Return the in-memory cached org policy without network fetch.
    fn current_org_policy(&self) -> Result<Option<OrgPolicy>, ServiceError>;
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
        self.trigger_background_sync();
        Ok(session)
    }

    /// Returns true if the agent process for the session is still running.
    pub fn is_session_running(&mut self, session_id: &str) -> bool {
        self.daemon.runner.is_session_running(session_id)
    }

    fn trigger_background_sync(&self) {
        let store = self.store.clone();
        std::thread::spawn(move || drain_audit_queue(&store));
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

        // If sync is configured, append any new audit events to the outbox.
        if state.sync_config.is_some() {
            let already_queued: std::collections::HashSet<u64> = state
                .audit_queue
                .iter()
                .filter(|entry| entry.session_id == session_id)
                .map(|entry| entry.event.sequence)
                .collect();
            let now = now_ms();
            for event in &record.events {
                if !already_queued.contains(&event.sequence) {
                    state.audit_queue.push(AuditQueueEntry {
                        session_id: session_id.to_string(),
                        event: event.clone(),
                        queued_at_ms: now,
                        sent: false,
                    });
                }
            }
        }

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
                    signature_status: policy_core::verify_signature(profile),
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
        // Load the cached org policy and pass it through only if it applies to
        // this (agent, project) pair. resolve_effective_policy is called inside
        // run_preflight via effective_profile_for.
        let state = self.store.load_state()?;
        let org_policy = state.cached_org_policy.as_ref().filter(|org| {
            policy_core::org_policy_applies(org, agent_id, project_dir)
        }).cloned();
        Ok(run_preflight(project_dir, &agent_tool, &profile, &capabilities, org_policy.as_ref()))
    }

    fn load_profile(&self, profile_id: &str) -> Result<ProfileDetail, ServiceError> {
        let state = self.store.load_state()?;
        // Custom profiles take precedence over presets.
        if let Some(custom) = state.custom_profiles.iter().find(|p| p.id == profile_id).cloned() {
            // Recompute signature_status from the stored signature on every load.
            let profile = custom.clone().into_profile();
            let mut detail = custom;
            detail.signature_status = policy_core::verify_signature(&profile);
            return Ok(detail);
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

    fn sign_profile(&self, profile_id: &str, signing_key_b64: &str) -> Result<(), ServiceError> {
        use base64::{engine::general_purpose, Engine as _};

        let key_bytes = general_purpose::STANDARD
            .decode(signing_key_b64)
            .map_err(|_| ServiceError::InvalidSigningKey)?;

        let mut state = self.store.load_state()?;
        let stored = state
            .custom_profiles
            .iter_mut()
            .find(|p| p.id == profile_id)
            .ok_or_else(|| ServiceError::PresetProfileNotSignable(profile_id.into()))?;

        let mut profile = stored.clone().into_profile();
        policy_core::sign_profile(&mut profile, "", &key_bytes)
            .map_err(ServiceError::ProfileSign)?;

        let signed_detail = ProfileDetail::from_profile(&profile);
        *stored = signed_detail;
        self.store.save_state(&state)?;
        Ok(())
    }

    fn load_remote_profile(&self, url: &str) -> Result<ProfileDetail, ServiceError> {
        let cache_dir = self.store.profile_cache_dir();
        let profile = fetch_remote_profile(url, &cache_dir)?;
        Ok(ProfileDetail::from_profile(&profile))
    }

    fn configure_sync(&self, config: SyncConfig) -> Result<(), ServiceError> {
        let mut state = self.store.load_state()?;
        state.sync_config = Some(config);
        self.store.save_state(&state)?;
        Ok(())
    }

    fn get_sync_status(&self) -> Result<SyncStatus, ServiceError> {
        let state = self.store.load_state()?;
        let queue_depth = state.audit_queue.iter().filter(|e| !e.sent).count();
        Ok(SyncStatus {
            configured: state.sync_config.is_some(),
            queue_depth,
            last_sync_at_ms: state.last_sync_at_ms,
            last_error: state.last_sync_error.clone(),
        })
    }

    fn sync_audit_events(&self) -> Result<SyncStatus, ServiceError> {
        drain_audit_queue(&self.store);
        self.get_sync_status()
    }

    fn configure_org_policy_url(&mut self, url: Option<String>) -> Result<(), ServiceError> {
        let mut state = self.store.load_state()?;
        state.org_policy_url = url;
        self.store.save_state(&state)?;
        Ok(())
    }

    fn fetch_org_policy(&mut self) -> Result<Option<OrgPolicy>, ServiceError> {
        let state = self.store.load_state()?;
        let url = match &state.org_policy_url {
            Some(u) => u,
            None => return Ok(None),
        };

        let cache_dir = self.store.org_policy_cache_dir();
        let policy: OrgPolicy = fetch_remote_cached(url, &cache_dir)?;

        let mut state = self.store.load_state()?;
        state.cached_org_policy = Some(policy.clone());
        self.store.save_state(&state)?;

        Ok(Some(policy))
    }

    fn current_org_policy(&self) -> Result<Option<OrgPolicy>, ServiceError> {
        let state = self.store.load_state()?;
        Ok(state.cached_org_policy.clone())
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

pub fn default_agents() -> Vec<AgentCatalogEntry> {
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

pub fn agent_tool_from_id(id: &str) -> AgentTool {
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

pub fn default_profiles() -> Vec<Profile> {
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
pub fn profiles_for_agent(agent_id: Option<&str>, project_root: &str) -> Vec<Profile> {
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
    #[error("profile signing failed: {0}")]
    ProfileSign(#[source] policy_core::SignError),
    #[error("invalid signing key (expected base64-encoded 32-byte ed25519 seed)")]
    InvalidSigningKey,
    #[error("only custom profiles can be signed; preset '{0}' is read-only")]
    PresetProfileNotSignable(String),
    #[error("remote profile fetch failed: {0}")]
    RemoteFetch(String),
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

#[cfg(test)]
mod tests {
    use super::*;
    use engine_greywall::GreywallAdapter;
    use policy_core::{
        DefaultAction, FilesystemPolicy, NetworkPolicy, OrgPolicy, OrgPolicyScope, Policy,
        ProcessPolicy, Profile,
    };

    fn make_profile(network_default: DefaultAction) -> Profile {
        Profile {
            id: "test-profile".into(),
            name: "Test".into(),
            description: None,
            extends: None,
            signature: None,
            policy: Policy {
                filesystem: FilesystemPolicy {
                    readable_roots: vec!["/project".into()],
                    writable_roots: vec!["/project".into()],
                    blocked_roots: vec![],
                },
                network: NetworkPolicy {
                    default_action: network_default,
                    allowed_hosts: vec!["api.example.com".into()],
                    blocked_hosts: vec![],
                },
                process: ProcessPolicy {
                    default_action: DefaultAction::Allow,
                    allowed_commands: vec![],
                    blocked_commands: vec![],
                },
            },
        }
    }

    fn make_org_policy(network_default: DefaultAction, scope: Option<OrgPolicyScope>) -> OrgPolicy {
        OrgPolicy {
            id: "org-policy".into(),
            name: "Org".into(),
            description: None,
            scope,
            signature: None,
            policy: Policy {
                filesystem: FilesystemPolicy {
                    readable_roots: vec![],
                    writable_roots: vec![],
                    blocked_roots: vec![],
                },
                network: NetworkPolicy {
                    default_action: network_default,
                    allowed_hosts: vec![],
                    blocked_hosts: vec![],
                },
                process: ProcessPolicy {
                    default_action: DefaultAction::Allow,
                    allowed_commands: vec![],
                    blocked_commands: vec![],
                },
            },
        }
    }

    fn make_capabilities() -> EngineCapabilitySnapshot {
        GreywallAdapter::default().capability_snapshot()
    }

    /// Test 1: org policy with stricter network default (Deny) applied to a local profile that
    /// allows network → the effective profile has network.default_action=Deny.  The diagnostic
    /// for the policy-compatibility check (or any new diagnostic) that appears only in the merged
    /// run should carry from_org_policy=true.
    ///
    /// Here we use a simpler observable: the "Org policy floor active" informational diagnostic
    /// is injected whenever an org policy applies, confirming the integration path ran.
    #[test]
    fn test_preflight_org_floor_stricter_default_annotated() {
        // Local profile allows network; org floor denies it.
        let local = make_profile(DefaultAction::Allow);
        let org = make_org_policy(DefaultAction::Deny, None); // no scope = applies to all

        let caps = make_capabilities();
        // project_dir doesn't exist — that's fine; we're testing annotation logic.
        let report = run_preflight("/nonexistent/project", &AgentTool::ClaudeCode, &local, &caps, Some(&org));

        // The org-floor notice must be present.
        let org_notice = report.diagnostics.iter().find(|d| d.label == "Org policy floor active");
        assert!(org_notice.is_some(), "expected 'Org policy floor active' diagnostic");

        // The from_org_policy notice itself must NOT be marked from_org_policy.
        assert!(!org_notice.unwrap().from_org_policy);

        // At least one diagnostic in the report must carry from_org_policy=true,
        // because the merged profile differs from the local profile.
        let org_marked = report.diagnostics.iter().any(|d| d.from_org_policy);
        assert!(org_marked, "expected at least one diagnostic with from_org_policy=true");
    }

    /// Test 2: org policy scoped to a different agent type → does NOT apply → preflight
    /// result is identical to local-only run (no org-marked diagnostics, no notice).
    #[test]
    fn test_preflight_org_scope_mismatch_no_org_diagnostics() {
        let local = make_profile(DefaultAction::Allow);
        // Scope restricted to "codex"; we're running "claude-code".
        let scope = OrgPolicyScope {
            agent_types: Some(vec!["codex".into()]),
            project_path_glob: None,
        };
        let org = make_org_policy(DefaultAction::Deny, Some(scope));

        let caps = make_capabilities();
        let report_with_org = run_preflight("/nonexistent/project", &AgentTool::ClaudeCode, &local, &caps, Some(&org));
        let report_local = run_preflight("/nonexistent/project", &AgentTool::ClaudeCode, &local, &caps, None);

        // No org-floor notice.
        let org_notice = report_with_org.diagnostics.iter().find(|d| d.label == "Org policy floor active");
        assert!(org_notice.is_none(), "org scope mismatch: should not inject org-floor notice");

        // No from_org_policy markers.
        let any_org_marked = report_with_org.diagnostics.iter().any(|d| d.from_org_policy);
        assert!(!any_org_marked, "org scope mismatch: no diagnostics should be from_org_policy");

        // Diagnostic count and labels match local-only run.
        assert_eq!(
            report_with_org.diagnostics.len(),
            report_local.diagnostics.len(),
            "org scope mismatch: diagnostic count should equal local-only run"
        );
    }
}
