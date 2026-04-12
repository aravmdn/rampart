use policy_core::{
    AuditEvent, AuditEventKind, AuditOutcome, CapabilitySupport, EngineCapabilitySnapshot,
    FilesystemCapabilitySnapshot, NetworkCapabilitySnapshot, ProcessCapabilitySnapshot,
    ViolationEvent, ViolationKind,
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
    BlockRead,
    BlockWrite,
    BlockExec,
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
            RawEngineEventKind::AllowRead => NormalizedEngineEvent {
                audit: AuditEvent {
                    session_id: event.session_id,
                    sequence: event.sequence,
                    occurred_at_ms: event.occurred_at_ms,
                    kind: AuditEventKind::OperationObserved,
                    outcome: AuditOutcome::Allowed,
                    message: format!("Allowed read inside policy scope: {}", event.target),
                    violation: None,
                },
                violation: None,
            },
            RawEngineEventKind::BlockRead => blocked_event(event, "read"),
            RawEngineEventKind::BlockWrite => blocked_event(event, "write"),
            RawEngineEventKind::BlockExec => blocked_event(event, "execute"),
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
    })
}

fn blocked_event(event: RawEngineEvent, action: &str) -> NormalizedEngineEvent {
    let violation = ViolationEvent {
        session_id: event.session_id.clone(),
        sequence: event.sequence,
        occurred_at_ms: event.occurred_at_ms,
        kind: ViolationKind::Filesystem,
        action: action.into(),
        target: event.target.clone(),
        rule_id: "fs.read.project-root-only".into(),
        rule_label: "Read access limited to selected project root.".into(),
        reason: "Policy denied access outside allowed project roots.".into(),
        platform_note: Some(
            "greywall is reference adapter only. Windows runtime support remains unverified.".into(),
        ),
    };

    NormalizedEngineEvent {
        audit: AuditEvent {
            session_id: event.session_id,
            sequence: event.sequence,
            occurred_at_ms: event.occurred_at_ms,
            kind: AuditEventKind::ViolationRecorded,
            outcome: AuditOutcome::Blocked,
            message: format!("Blocked {action}: {}", event.target),
            violation: Some(violation.clone()),
        },
        violation: Some(violation),
    }
}

fn current_platform_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        return "windows";
    }
    #[cfg(target_os = "macos")]
    {
        return "macos";
    }
    #[cfg(target_os = "linux")]
    {
        return "linux";
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
        assert_eq!(violation.rule_id, "fs.read.project-root-only");
        assert!(violation.platform_note.is_some());
    }
}
