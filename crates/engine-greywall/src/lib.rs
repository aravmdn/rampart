use policy_core::{
    AuditEvent, AuditEventCategory, AuditEventKind, AuditOutcome, CapabilitySupport,
    EngineCapabilitySnapshot, FilesystemCapabilitySnapshot, NetworkCapabilitySnapshot,
    PlatformLimitation, ProcessCapabilitySnapshot, ViolationEvent, ViolationExplanation,
    ViolationKind,
};
use serde::Deserialize;
use std::path::PathBuf;
use thiserror::Error;

pub const ENGINE_ID: &str = "greywall";

pub trait EnforcementEngine: Send + Sync {
    fn engine_id(&self) -> &'static str;
    fn capability_snapshot(&self) -> EngineCapabilitySnapshot;
    fn normalize_event(&self, event: RawEngineEvent) -> NormalizedEngineEvent;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreywallAdapter {
    pub binary_path: PathBuf,
    pub version: GreywallVersion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GreywallVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawEngineEventKind {
    AllowRead,
    AllowWrite,
    AllowExec,
    AllowNetwork,
    BlockRead,
    BlockWrite,
    BlockExec,
    BlockNetwork,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawEngineEvent {
    pub session_id: String,
    pub sequence: u64,
    pub occurred_at_ms: u64,
    pub kind: RawEngineEventKind,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedEngineEvent {
    pub audit: AuditEvent,
    pub violation: Option<ViolationEvent>,
}

#[derive(Debug, Deserialize)]
struct BlockedFixture {
    occurred_at_ms: u64,
    operation: String,
    target: String,
    reason: String,
    policy_rule_id: String,
    policy_rule_label: String,
    platform_note: Option<String>,
}

impl GreywallAdapter {
    pub fn from_binary_path(binary_path: PathBuf, version: GreywallVersion) -> Result<Self, EngineError> {
        if binary_path.as_os_str().is_empty() {
            return Err(EngineError::MissingBinaryPath);
        }
        if version.major == 0 && version.minor < 3 {
            return Err(EngineError::UnsupportedVersion(version.render()));
        }
        Ok(Self { binary_path, version })
    }

    pub fn discover() -> Result<Self, EngineError> {
        Self::from_binary_path(default_binary_path_for_current_platform(), GreywallVersion { major: 0, minor: 3, patch: 0 })
    }

    fn build_capability_snapshot(&self) -> EngineCapabilitySnapshot {
        let _ = &self.binary_path;
        let platform = current_platform_name();
        let windows_like = platform == "windows" || platform == "other";

        EngineCapabilitySnapshot {
            engine_name: ENGINE_ID.into(),
            engine_version: Some(self.version.render()),
            platform: platform.into(),
            filesystem: FilesystemCapabilitySnapshot {
                enforcement: if windows_like {
                    CapabilitySupport::Unsupported
                } else {
                    CapabilitySupport::Supported
                },
                observation: if windows_like {
                    CapabilitySupport::Unsupported
                } else {
                    CapabilitySupport::Supported
                },
                temporary_file_coverage: if windows_like {
                    CapabilitySupport::Unsupported
                } else {
                    CapabilitySupport::Limited
                },
                atomic_rename_coverage: if windows_like {
                    CapabilitySupport::Unsupported
                } else {
                    CapabilitySupport::Limited
                },
            },
            network: NetworkCapabilitySnapshot {
                enforcement: if windows_like {
                    CapabilitySupport::Unsupported
                } else {
                    CapabilitySupport::Limited
                },
                observation: if windows_like {
                    CapabilitySupport::Unsupported
                } else {
                    CapabilitySupport::Limited
                },
                proxy_awareness: if windows_like {
                    CapabilitySupport::Unsupported
                } else {
                    CapabilitySupport::Limited
                },
            },
            process: ProcessCapabilitySnapshot {
                enforcement: if windows_like {
                    CapabilitySupport::Unsupported
                } else {
                    CapabilitySupport::Supported
                },
                observation: if windows_like {
                    CapabilitySupport::Unsupported
                } else {
                    CapabilitySupport::Supported
                },
                termination: if windows_like {
                    CapabilitySupport::Unsupported
                } else {
                    CapabilitySupport::Supported
                },
            },
        }
    }

    fn map_event(&self, event: RawEngineEvent) -> NormalizedEngineEvent {
        let _ = &self.version;
        match event.kind {
            RawEngineEventKind::AllowRead => allowed_event(
                event,
                AuditEventKind::FilesystemAllowed,
                "Allowed read inside policy scope",
            ),
            RawEngineEventKind::AllowWrite => allowed_event(
                event,
                AuditEventKind::FilesystemAllowed,
                "Allowed write inside policy scope",
            ),
            RawEngineEventKind::AllowExec => allowed_event(
                event,
                AuditEventKind::ProcessAllowed,
                "Allowed process execution inside policy scope",
            ),
            RawEngineEventKind::AllowNetwork => allowed_event(
                event,
                AuditEventKind::NetworkAllowed,
                "Allowed network access inside policy scope",
            ),
            RawEngineEventKind::BlockRead => blocked_filesystem_event(event, "read"),
            RawEngineEventKind::BlockWrite => blocked_filesystem_event(event, "write"),
            RawEngineEventKind::BlockExec => blocked_process_event(event),
            RawEngineEventKind::BlockNetwork => blocked_network_event(event),
        }
    }
}

impl EnforcementEngine for GreywallAdapter {
    fn engine_id(&self) -> &'static str {
        ENGINE_ID
    }

    fn capability_snapshot(&self) -> EngineCapabilitySnapshot {
        self.build_capability_snapshot()
    }

    fn normalize_event(&self, event: RawEngineEvent) -> NormalizedEngineEvent {
        self.map_event(event)
    }
}

pub fn parse_blocked_fixture(session_id: &str, raw: &str) -> Result<ViolationEvent, EngineError> {
    let fixture: BlockedFixture =
        serde_json::from_str(raw).map_err(|error| EngineError::FixtureParse(error.to_string()))?;

    let action = match fixture.operation.as_str() {
        "read" | "write" | "execute" => fixture.operation,
        other => return Err(EngineError::UnsupportedOperation(other.to_string())),
    };

    Ok(ViolationEvent {
        session_id: session_id.into(),
        sequence: 1,
        occurred_at_ms: fixture.occurred_at_ms,
        kind: ViolationKind::Filesystem,
        action,
        target: fixture.target,
        rule_id: fixture.policy_rule_id,
        rule_label: fixture.policy_rule_label,
        reason: fixture.reason,
        platform_note: fixture.platform_note,
        explanation: None,
    })
}

fn allowed_event(
    event: RawEngineEvent,
    kind: AuditEventKind,
    label: &str,
) -> NormalizedEngineEvent {
    NormalizedEngineEvent {
        audit: AuditEvent {
            session_id: event.session_id,
            sequence: event.sequence,
            occurred_at_ms: event.occurred_at_ms,
            kind,
            category: AuditEventCategory::PolicyEnforcement,
            outcome: AuditOutcome::Allowed,
            message: format!("{label}: {}", event.target),
            violation: None,
        },
        violation: None,
    }
}

fn greywall_platform_limitation() -> PlatformLimitation {
    PlatformLimitation {
        platform: current_platform_name().into(),
        engine: ENGINE_ID.into(),
        detail: "greywall is the reference adapter only. Windows runtime enforcement support \
                 remains unverified."
            .into(),
    }
}

fn blocked_filesystem_event(event: RawEngineEvent, action: &str) -> NormalizedEngineEvent {
    let explanation = ViolationExplanation {
        rule_description: "Access outside the allowed project roots is denied by the filesystem \
                            scope policy."
            .into(),
        platform_limitation: Some(greywall_platform_limitation()),
        remediation_hint: Some(
            "Adjust the profile's filesystem.readable_roots or filesystem.writable_roots to \
             include the target path."
                .into(),
        ),
    };
    let violation = ViolationEvent {
        session_id: event.session_id.clone(),
        sequence: event.sequence,
        occurred_at_ms: event.occurred_at_ms,
        kind: ViolationKind::Filesystem,
        action: action.into(),
        target: event.target.clone(),
        rule_id: "fs.scope.project-root-only".into(),
        rule_label: "Filesystem access limited to selected project root.".into(),
        reason: "Policy denied access outside allowed project roots.".into(),
        platform_note: Some(
            "greywall is the reference adapter only. Windows runtime support remains unverified."
                .into(),
        ),
        explanation: Some(explanation),
    };
    NormalizedEngineEvent {
        audit: AuditEvent {
            session_id: event.session_id,
            sequence: event.sequence,
            occurred_at_ms: event.occurred_at_ms,
            kind: AuditEventKind::FilesystemBlocked,
            category: AuditEventCategory::PolicyEnforcement,
            outcome: AuditOutcome::Blocked,
            message: format!("Blocked filesystem {action}: {}", event.target),
            violation: Some(violation.clone()),
        },
        violation: Some(violation),
    }
}

fn blocked_process_event(event: RawEngineEvent) -> NormalizedEngineEvent {
    let explanation = ViolationExplanation {
        rule_description: "Process execution outside the allowed command list is denied by the \
                            process policy."
            .into(),
        platform_limitation: Some(greywall_platform_limitation()),
        remediation_hint: Some(
            "Add the command to the profile's process.allowed_commands list.".into(),
        ),
    };
    let violation = ViolationEvent {
        session_id: event.session_id.clone(),
        sequence: event.sequence,
        occurred_at_ms: event.occurred_at_ms,
        kind: ViolationKind::Process,
        action: "execute".into(),
        target: event.target.clone(),
        rule_id: "process.scope.allowed-commands-only".into(),
        rule_label: "Process execution limited to allowed commands list.".into(),
        reason: "Policy denied process execution not in allowed commands list.".into(),
        platform_note: Some(
            "greywall is the reference adapter only. Windows runtime support remains unverified."
                .into(),
        ),
        explanation: Some(explanation),
    };
    NormalizedEngineEvent {
        audit: AuditEvent {
            session_id: event.session_id,
            sequence: event.sequence,
            occurred_at_ms: event.occurred_at_ms,
            kind: AuditEventKind::ProcessBlocked,
            category: AuditEventCategory::PolicyEnforcement,
            outcome: AuditOutcome::Blocked,
            message: format!("Blocked process execution: {}", event.target),
            violation: Some(violation.clone()),
        },
        violation: Some(violation),
    }
}

fn blocked_network_event(event: RawEngineEvent) -> NormalizedEngineEvent {
    let explanation = ViolationExplanation {
        rule_description: "Network access to this host is denied by the network policy.".into(),
        platform_limitation: Some(greywall_platform_limitation()),
        remediation_hint: Some(
            "Add the host to the profile's network.allowed_hosts list.".into(),
        ),
    };
    let violation = ViolationEvent {
        session_id: event.session_id.clone(),
        sequence: event.sequence,
        occurred_at_ms: event.occurred_at_ms,
        kind: ViolationKind::Network,
        action: "network".into(),
        target: event.target.clone(),
        rule_id: "network.egress.allowed-hosts-only".into(),
        rule_label: "Network access limited to allowed hosts.".into(),
        reason: "Policy denied network access to host not in allowed list.".into(),
        platform_note: Some(
            "greywall is the reference adapter only. Windows runtime support remains unverified."
                .into(),
        ),
        explanation: Some(explanation),
    };
    NormalizedEngineEvent {
        audit: AuditEvent {
            session_id: event.session_id,
            sequence: event.sequence,
            occurred_at_ms: event.occurred_at_ms,
            kind: AuditEventKind::NetworkBlocked,
            category: AuditEventCategory::PolicyEnforcement,
            outcome: AuditOutcome::Blocked,
            message: format!("Blocked network access: {}", event.target),
            violation: Some(violation.clone()),
        },
        violation: Some(violation),
    }
}

fn current_platform_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        "other"
    }
}

fn default_binary_path_for_current_platform() -> PathBuf {
    if cfg!(target_os = "windows") {
        PathBuf::from("greywall.exe")
    } else {
        PathBuf::from("greywall")
    }
}

impl GreywallVersion {
    pub fn render(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EngineError {
    #[error("greywall binary path missing")]
    MissingBinaryPath,
    #[error("greywall version '{0}' unsupported")]
    UnsupportedVersion(String),
    #[error("blocked verification fixture parse failed: {0}")]
    FixtureParse(String),
    #[error("blocked verification fixture used unsupported operation: {0}")]
    UnsupportedOperation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_old_versions() {
        let error = GreywallAdapter::from_binary_path(
            PathBuf::from("greywall"),
            GreywallVersion { major: 0, minor: 2, patch: 9 },
        )
        .expect_err("old version must fail");

        assert_eq!(error, EngineError::UnsupportedVersion("0.2.9".into()));
    }

    #[test]
    fn reports_truthful_windows_snapshot() {
        let adapter = GreywallAdapter::discover().expect("discovery should work");
        let snapshot = adapter.capability_snapshot();

        if snapshot.platform == "windows" {
            assert_eq!(snapshot.filesystem.enforcement, CapabilitySupport::Unsupported);
            assert_eq!(snapshot.network.enforcement, CapabilitySupport::Unsupported);
        }
    }

    #[test]
    fn normalizes_block_event_with_rule_metadata() {
        let adapter = GreywallAdapter::discover().expect("discovery should work");
        let batch = adapter.normalize_event(RawEngineEvent {
            session_id: "session-1".into(),
            sequence: 2,
            occurred_at_ms: 200,
            kind: RawEngineEventKind::BlockRead,
            target: r"C:\Users\alice\.ssh\id_rsa".into(),
        });

        let violation = batch.violation.expect("violation required");
        assert_eq!(violation.rule_id, "fs.scope.project-root-only");
        assert!(violation.platform_note.is_some());
    }
}
