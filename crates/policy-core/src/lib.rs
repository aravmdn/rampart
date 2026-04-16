use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum DefaultAction {
    Allow,
    #[default]
    Deny,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilitySupport {
    Unsupported,
    Limited,
    Supported,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformLimitation {
    pub platform: String,
    pub engine: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViolationExplanation {
    pub rule_description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_limitation: Option<PlatformLimitation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation_hint: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationErrorCode {
    MissingValue,
    InvalidValue,
    DuplicateValue,
    ConflictingValue,
    UnsupportedCapability,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ValidationError {
    pub code: ValidationErrorCode,
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValidationErrors {
    pub items: Vec<ValidationError>,
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.items.is_empty() {
            return write!(f, "validation failed");
        }

        let rendered = self
            .items
            .iter()
            .map(|item| format!("{}: {}", item.field, item.message))
            .collect::<Vec<_>>()
            .join("; ");
        write!(f, "{rendered}")
    }
}

impl Error for ValidationErrors {}

impl ValidationErrors {
    pub fn push(
        &mut self,
        code: ValidationErrorCode,
        field: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.items.push(ValidationError {
            code,
            field: field.into(),
            message: message.into(),
        });
    }

    pub fn extend(&mut self, mut other: ValidationErrors) {
        self.items.append(&mut other.items);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum AgentTool {
    ClaudeCode,
    Codex,
    Cursor,
    Copilot,
    Aider,
    Goose,
    OpenCode,
    GeminiCli,
    Custom {
        id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        display_name: Option<String>,
    },
}

impl AgentTool {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::default();

        if let AgentTool::Custom { id, .. } = self {
            let trimmed = id.trim();
            if trimmed.is_empty() {
                errors.push(
                    ValidationErrorCode::InvalidValue,
                    "agent_tool.id",
                    "custom agent tool id must not be empty",
                );
            } else if !trimmed
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
            {
                errors.push(
                    ValidationErrorCode::InvalidValue,
                    "agent_tool.id",
                    "custom agent tool id must use ascii letters, digits, dash, or underscore",
                );
            }
        }

        finish(errors)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FilesystemPolicy {
    #[serde(default)]
    pub readable_roots: Vec<String>,
    #[serde(default)]
    pub writable_roots: Vec<String>,
    #[serde(default)]
    pub blocked_roots: Vec<String>,
}

impl FilesystemPolicy {
    fn validate_with_prefix(&self, prefix: &str) -> ValidationErrors {
        let mut errors = ValidationErrors::default();

        validate_string_list(
            &format!("{prefix}.readable_roots"),
            &self.readable_roots,
            &mut errors,
        );
        validate_string_list(
            &format!("{prefix}.writable_roots"),
            &self.writable_roots,
            &mut errors,
        );
        validate_string_list(
            &format!("{prefix}.blocked_roots"),
            &self.blocked_roots,
            &mut errors,
        );

        validate_conflict(
            prefix,
            &self.readable_roots,
            &self.blocked_roots,
            "filesystem root cannot be both readable and blocked",
            &mut errors,
        );
        validate_conflict(
            prefix,
            &self.writable_roots,
            &self.blocked_roots,
            "filesystem root cannot be both writable and blocked",
            &mut errors,
        );

        errors
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct NetworkPolicy {
    #[serde(default)]
    pub default_action: DefaultAction,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    #[serde(default)]
    pub blocked_hosts: Vec<String>,
}

impl NetworkPolicy {
    fn validate_with_prefix(&self, prefix: &str) -> ValidationErrors {
        let mut errors = ValidationErrors::default();

        validate_string_list(
            &format!("{prefix}.allowed_hosts"),
            &self.allowed_hosts,
            &mut errors,
        );
        validate_string_list(
            &format!("{prefix}.blocked_hosts"),
            &self.blocked_hosts,
            &mut errors,
        );
        validate_conflict(
            prefix,
            &self.allowed_hosts,
            &self.blocked_hosts,
            "host cannot be both allowed and blocked",
            &mut errors,
        );

        errors
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ProcessPolicy {
    #[serde(default)]
    pub default_action: DefaultAction,
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    #[serde(default)]
    pub blocked_commands: Vec<String>,
}

impl ProcessPolicy {
    fn validate_with_prefix(&self, prefix: &str) -> ValidationErrors {
        let mut errors = ValidationErrors::default();

        validate_string_list(
            &format!("{prefix}.allowed_commands"),
            &self.allowed_commands,
            &mut errors,
        );
        validate_string_list(
            &format!("{prefix}.blocked_commands"),
            &self.blocked_commands,
            &mut errors,
        );
        validate_conflict(
            prefix,
            &self.allowed_commands,
            &self.blocked_commands,
            "command cannot be both allowed and blocked",
            &mut errors,
        );

        errors
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Policy {
    #[serde(default)]
    pub filesystem: FilesystemPolicy,
    #[serde(default)]
    pub network: NetworkPolicy,
    #[serde(default)]
    pub process: ProcessPolicy,
}

impl Policy {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::default();
        errors.extend(self.filesystem.validate_with_prefix("policy.filesystem"));
        errors.extend(self.network.validate_with_prefix("policy.network"));
        errors.extend(self.process.validate_with_prefix("policy.process"));
        finish(errors)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extends: Option<String>,
    pub policy: Policy,
}

impl Profile {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::default();

        validate_required("profile.id", &self.id, "profile id", &mut errors);
        validate_required("profile.name", &self.name, "profile name", &mut errors);

        if let Some(parent) = &self.extends {
            validate_required("profile.extends", parent, "base profile id", &mut errors);
            if parent.trim() == self.id.trim() && !parent.trim().is_empty() {
                errors.push(
                    ValidationErrorCode::ConflictingValue,
                    "profile.extends",
                    "profile cannot extend itself",
                );
            }
        }

        if let Err(policy_errors) = self.policy.validate() {
            errors.extend(policy_errors);
        }

        finish(errors)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CompiledFilesystemPolicy {
    #[serde(default)]
    pub readable_roots: Vec<String>,
    #[serde(default)]
    pub writable_roots: Vec<String>,
    #[serde(default)]
    pub blocked_roots: Vec<String>,
}

impl CompiledFilesystemPolicy {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        finish(
            FilesystemPolicy {
                readable_roots: self.readable_roots.clone(),
                writable_roots: self.writable_roots.clone(),
                blocked_roots: self.blocked_roots.clone(),
            }
            .validate_with_prefix("compiled_policy.filesystem"),
        )
    }
}

impl From<FilesystemPolicy> for CompiledFilesystemPolicy {
    fn from(value: FilesystemPolicy) -> Self {
        Self {
            readable_roots: normalize_list(value.readable_roots),
            writable_roots: normalize_list(value.writable_roots),
            blocked_roots: normalize_list(value.blocked_roots),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CompiledNetworkPolicy {
    pub default_action: DefaultAction,
    #[serde(default)]
    pub allowed_hosts: Vec<String>,
    #[serde(default)]
    pub blocked_hosts: Vec<String>,
}

impl CompiledNetworkPolicy {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        finish(
            NetworkPolicy {
                default_action: self.default_action,
                allowed_hosts: self.allowed_hosts.clone(),
                blocked_hosts: self.blocked_hosts.clone(),
            }
            .validate_with_prefix("compiled_policy.network"),
        )
    }
}

impl From<NetworkPolicy> for CompiledNetworkPolicy {
    fn from(value: NetworkPolicy) -> Self {
        Self {
            default_action: value.default_action,
            allowed_hosts: normalize_list(value.allowed_hosts),
            blocked_hosts: normalize_list(value.blocked_hosts),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct CompiledProcessPolicy {
    pub default_action: DefaultAction,
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    #[serde(default)]
    pub blocked_commands: Vec<String>,
}

impl CompiledProcessPolicy {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        finish(
            ProcessPolicy {
                default_action: self.default_action,
                allowed_commands: self.allowed_commands.clone(),
                blocked_commands: self.blocked_commands.clone(),
            }
            .validate_with_prefix("compiled_policy.process"),
        )
    }
}

impl From<ProcessPolicy> for CompiledProcessPolicy {
    fn from(value: ProcessPolicy) -> Self {
        Self {
            default_action: value.default_action,
            allowed_commands: normalize_list(value.allowed_commands),
            blocked_commands: normalize_list(value.blocked_commands),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompiledPolicy {
    pub filesystem: CompiledFilesystemPolicy,
    pub network: CompiledNetworkPolicy,
    pub process: CompiledProcessPolicy,
}

impl CompiledPolicy {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::default();
        if let Err(inner) = self.filesystem.validate() {
            errors.extend(inner);
        }
        if let Err(inner) = self.network.validate() {
            errors.extend(inner);
        }
        if let Err(inner) = self.process.validate() {
            errors.extend(inner);
        }
        finish(errors)
    }
}

pub fn compile_policy(policy: &Policy) -> Result<CompiledPolicy, ValidationErrors> {
    policy.validate()?;

    Ok(CompiledPolicy {
        filesystem: policy.filesystem.clone().into(),
        network: policy.network.clone().into(),
        process: policy.process.clone().into(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SessionStatus {
    Pending,
    Running,
    Finished,
    Failed,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub project_path: String,
    pub agent_tool: AgentTool,
    pub profile_id: String,
    pub compiled_policy: CompiledPolicy,
    pub status: SessionStatus,
    pub started_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at_ms: Option<u64>,
}

impl Session {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::default();
        validate_required("session.id", &self.id, "session id", &mut errors);
        validate_required(
            "session.project_path",
            &self.project_path,
            "session project path",
            &mut errors,
        );
        validate_required(
            "session.profile_id",
            &self.profile_id,
            "session profile id",
            &mut errors,
        );

        if let Err(inner) = self.agent_tool.validate() {
            errors.extend(inner);
        }
        if let Err(inner) = self.compiled_policy.validate() {
            errors.extend(inner);
        }

        if let Some(ended_at_ms) = self.ended_at_ms {
            if ended_at_ms < self.started_at_ms {
                errors.push(
                    ValidationErrorCode::ConflictingValue,
                    "session.ended_at_ms",
                    "session end time must not be earlier than start time",
                );
            }
        }

        finish(errors)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ViolationKind {
    Filesystem,
    Network,
    Process,
    Capability,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViolationEvent {
    pub session_id: String,
    pub sequence: u64,
    pub occurred_at_ms: u64,
    pub kind: ViolationKind,
    pub action: String,
    pub target: String,
    pub rule_id: String,
    pub rule_label: String,
    pub reason: String,
    pub platform_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation: Option<ViolationExplanation>,
}

impl ViolationEvent {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::default();
        validate_required(
            "violation_event.session_id",
            &self.session_id,
            "violation session id",
            &mut errors,
        );
        validate_required(
            "violation_event.action",
            &self.action,
            "violation action",
            &mut errors,
        );
        validate_required(
            "violation_event.target",
            &self.target,
            "violation target",
            &mut errors,
        );
        validate_required(
            "violation_event.rule_id",
            &self.rule_id,
            "violation rule id",
            &mut errors,
        );
        validate_required(
            "violation_event.rule_label",
            &self.rule_label,
            "violation rule label",
            &mut errors,
        );
        validate_required(
            "violation_event.reason",
            &self.reason,
            "violation reason",
            &mut errors,
        );
        if let Some(platform_note) = &self.platform_note {
            validate_required(
                "violation_event.platform_note",
                platform_note,
                "violation platform note",
                &mut errors,
            );
        }
        finish(errors)
    }
}

/// Broad category for filtering and display. Serialized as kebab-case.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AuditEventCategory {
    SessionLifecycle,
    #[default]
    PolicyEnforcement,
    SystemAlert,
}

/// Specific event kind. Domain-specific variants (Filesystem*, Network*, Process*)
/// are preferred for new events; the generic variants are kept for compatibility
/// with persisted records written before the taxonomy was expanded.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AuditEventKind {
    // Session lifecycle
    SessionLaunched,
    SessionEnded,
    PolicyCompiled,
    // Domain-specific operation outcomes (preferred)
    FilesystemAllowed,
    FilesystemBlocked,
    NetworkAllowed,
    NetworkBlocked,
    ProcessAllowed,
    ProcessBlocked,
    // Generic fallbacks (kept for backward compatibility)
    OperationObserved,
    ViolationRecorded,
    AlertRaised,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AuditOutcome {
    Allowed,
    Blocked,
    Info,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditEvent {
    pub session_id: String,
    pub sequence: u64,
    pub occurred_at_ms: u64,
    pub kind: AuditEventKind,
    #[serde(default)]
    pub category: AuditEventCategory,
    pub outcome: AuditOutcome,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub violation: Option<ViolationEvent>,
}

impl AuditEvent {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::default();
        validate_required(
            "audit_event.session_id",
            &self.session_id,
            "audit session id",
            &mut errors,
        );
        validate_required(
            "audit_event.message",
            &self.message,
            "audit message",
            &mut errors,
        );

        if let Some(violation) = &self.violation {
            if let Err(inner) = violation.validate() {
                errors.extend(inner);
            }
            if violation.session_id != self.session_id {
                errors.push(
                    ValidationErrorCode::ConflictingValue,
                    "audit_event.violation.session_id",
                    "audit event and nested violation must reference same session",
                );
            }
        }

        finish(errors)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FilesystemCapabilitySnapshot {
    pub enforcement: CapabilitySupport,
    pub observation: CapabilitySupport,
    pub temporary_file_coverage: CapabilitySupport,
    pub atomic_rename_coverage: CapabilitySupport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NetworkCapabilitySnapshot {
    pub enforcement: CapabilitySupport,
    pub observation: CapabilitySupport,
    pub proxy_awareness: CapabilitySupport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProcessCapabilitySnapshot {
    pub enforcement: CapabilitySupport,
    pub observation: CapabilitySupport,
    pub termination: CapabilitySupport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EngineCapabilitySnapshot {
    pub engine_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine_version: Option<String>,
    pub platform: String,
    pub filesystem: FilesystemCapabilitySnapshot,
    pub network: NetworkCapabilitySnapshot,
    pub process: ProcessCapabilitySnapshot,
}

impl EngineCapabilitySnapshot {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut errors = ValidationErrors::default();
        validate_required(
            "capabilities.engine_name",
            &self.engine_name,
            "engine name",
            &mut errors,
        );
        validate_required(
            "capabilities.platform",
            &self.platform,
            "platform name",
            &mut errors,
        );
        finish(errors)
    }
}

pub fn validate_policy_against_capabilities(
    policy: &Policy,
    capabilities: &EngineCapabilitySnapshot,
) -> Result<(), ValidationErrors> {
    let mut errors = ValidationErrors::default();

    if let Err(inner) = policy.validate() {
        errors.extend(inner);
    }
    if let Err(inner) = capabilities.validate() {
        errors.extend(inner);
    }

    if has_any(&policy.filesystem.readable_roots)
        || has_any(&policy.filesystem.writable_roots)
        || has_any(&policy.filesystem.blocked_roots)
    {
        require_support(
            capabilities.filesystem.enforcement,
            "capabilities.filesystem.enforcement",
            "filesystem policy needs filesystem enforcement support",
            &mut errors,
        );
    }

    if has_any(&policy.network.allowed_hosts)
        || has_any(&policy.network.blocked_hosts)
        || policy.network.default_action == DefaultAction::Allow
    {
        require_support(
            capabilities.network.enforcement,
            "capabilities.network.enforcement",
            "network policy needs network enforcement support",
            &mut errors,
        );
    }

    if has_any(&policy.process.allowed_commands)
        || has_any(&policy.process.blocked_commands)
        || policy.process.default_action == DefaultAction::Allow
    {
        require_support(
            capabilities.process.enforcement,
            "capabilities.process.enforcement",
            "process policy needs process enforcement support",
            &mut errors,
        );
    }

    finish(errors)
}

/// Agent-specific profile presets. Returns 2 presets tailored to the given agent.
/// Falls back to [`desktop_profile_presets`] for agents without specific presets.
pub fn agent_profile_presets(tool: &AgentTool, project_root: &str) -> Vec<Profile> {
    let normalized_root = project_root.trim().trim_end_matches(['\\', '/']).to_string();
    let user_profile = std::env::var("USERPROFILE").unwrap_or_else(|_| r"C:\Users\user".into());

    match tool {
        AgentTool::ClaudeCode => {
            let claude_config = format!("{user_profile}\\.claude");
            vec![
                Profile {
                    id: "claude-code.standard".into(),
                    name: "Claude Code Standard".into(),
                    description: Some(
                        "Project read/write, Claude config readable, Anthropic API allowed.".into(),
                    ),
                    extends: None,
                    policy: Policy {
                        filesystem: FilesystemPolicy {
                            readable_roots: vec![normalized_root.clone(), claude_config],
                            writable_roots: vec![normalized_root.clone()],
                            blocked_roots: vec![],
                        },
                        network: NetworkPolicy {
                            default_action: DefaultAction::Deny,
                            allowed_hosts: vec!["api.anthropic.com".into()],
                            blocked_hosts: vec![],
                        },
                        process: ProcessPolicy {
                            default_action: DefaultAction::Deny,
                            allowed_commands: vec![
                                "git".into(),
                                "node".into(),
                                "npm".into(),
                                "pnpm".into(),
                            ],
                            blocked_commands: vec!["powershell".into()],
                        },
                    },
                },
                Profile {
                    id: "claude-code.strict".into(),
                    name: "Claude Code Strict".into(),
                    description: Some(
                        "Project read/write only. Network denied. Child processes denied except git.".into(),
                    ),
                    extends: None,
                    policy: Policy {
                        filesystem: FilesystemPolicy {
                            readable_roots: vec![normalized_root.clone()],
                            writable_roots: vec![normalized_root],
                            blocked_roots: vec![r"C:\Users".into(), r"C:\Windows".into()],
                        },
                        network: NetworkPolicy {
                            default_action: DefaultAction::Deny,
                            allowed_hosts: vec![],
                            blocked_hosts: vec![],
                        },
                        process: ProcessPolicy {
                            default_action: DefaultAction::Deny,
                            allowed_commands: vec!["git".into()],
                            blocked_commands: vec!["powershell".into(), "cmd".into()],
                        },
                    },
                },
            ]
        }
        AgentTool::Codex => vec![
            Profile {
                id: "codex.standard".into(),
                name: "Codex Standard".into(),
                description: Some(
                    "Project read/write, OpenAI API allowed. git, node, npm permitted.".into(),
                ),
                extends: None,
                policy: Policy {
                    filesystem: FilesystemPolicy {
                        readable_roots: vec![normalized_root.clone()],
                        writable_roots: vec![normalized_root.clone()],
                        blocked_roots: vec![],
                    },
                    network: NetworkPolicy {
                        default_action: DefaultAction::Deny,
                        allowed_hosts: vec!["api.openai.com".into()],
                        blocked_hosts: vec![],
                    },
                    process: ProcessPolicy {
                        default_action: DefaultAction::Deny,
                        allowed_commands: vec!["git".into(), "node".into(), "npm".into()],
                        blocked_commands: vec!["powershell".into()],
                    },
                },
            },
            Profile {
                id: "codex.strict".into(),
                name: "Codex Strict".into(),
                description: Some(
                    "Project read/write only. Network denied. Only git permitted.".into(),
                ),
                extends: None,
                policy: Policy {
                    filesystem: FilesystemPolicy {
                        readable_roots: vec![normalized_root.clone()],
                        writable_roots: vec![normalized_root],
                        blocked_roots: vec![r"C:\Users".into(), r"C:\Windows".into()],
                    },
                    network: NetworkPolicy {
                        default_action: DefaultAction::Deny,
                        allowed_hosts: vec![],
                        blocked_hosts: vec![],
                    },
                    process: ProcessPolicy {
                        default_action: DefaultAction::Deny,
                        allowed_commands: vec!["git".into()],
                        blocked_commands: vec!["powershell".into(), "cmd".into()],
                    },
                },
            },
        ],
        AgentTool::Aider => vec![
            Profile {
                id: "aider.standard".into(),
                name: "Aider Standard".into(),
                description: Some(
                    "Project read/write, Anthropic and OpenAI APIs allowed. git and python permitted.".into(),
                ),
                extends: None,
                policy: Policy {
                    filesystem: FilesystemPolicy {
                        readable_roots: vec![normalized_root.clone()],
                        writable_roots: vec![normalized_root.clone()],
                        blocked_roots: vec![],
                    },
                    network: NetworkPolicy {
                        default_action: DefaultAction::Deny,
                        allowed_hosts: vec![
                            "api.anthropic.com".into(),
                            "api.openai.com".into(),
                        ],
                        blocked_hosts: vec![],
                    },
                    process: ProcessPolicy {
                        default_action: DefaultAction::Deny,
                        allowed_commands: vec![
                            "git".into(),
                            "python".into(),
                            "python3".into(),
                            "pip".into(),
                        ],
                        blocked_commands: vec!["powershell".into()],
                    },
                },
            },
            Profile {
                id: "aider.strict".into(),
                name: "Aider Strict".into(),
                description: Some(
                    "Project read/write only. Network denied. Only git permitted.".into(),
                ),
                extends: None,
                policy: Policy {
                    filesystem: FilesystemPolicy {
                        readable_roots: vec![normalized_root.clone()],
                        writable_roots: vec![normalized_root],
                        blocked_roots: vec![r"C:\Users".into(), r"C:\Windows".into()],
                    },
                    network: NetworkPolicy {
                        default_action: DefaultAction::Deny,
                        allowed_hosts: vec![],
                        blocked_hosts: vec![],
                    },
                    process: ProcessPolicy {
                        default_action: DefaultAction::Deny,
                        allowed_commands: vec!["git".into()],
                        blocked_commands: vec!["powershell".into(), "cmd".into()],
                    },
                },
            },
        ],
        _ => desktop_profile_presets(project_root),
    }
}

pub fn desktop_profile_presets(project_root: &str) -> Vec<Profile> {
    let normalized_root = project_root.trim().trim_end_matches(['\\', '/']).to_string();
    let apps_root = format!("{normalized_root}\\apps");
    let crates_root = format!("{normalized_root}\\crates");

    vec![
        Profile {
            id: "windows-safe".into(),
            name: "Windows Safe".into(),
            description: Some("Project scoped read/write with network denied by default.".into()),
            extends: None,
            policy: Policy {
                filesystem: FilesystemPolicy {
                    readable_roots: vec![normalized_root.clone()],
                    writable_roots: vec![apps_root],
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
        },
        Profile {
            id: "windows-strict".into(),
            name: "Windows Strict".into(),
            description: Some("Project read only outside source tree. Child process creation denied.".into()),
            extends: Some("windows-safe".into()),
            policy: Policy {
                filesystem: FilesystemPolicy {
                    readable_roots: vec![normalized_root],
                    writable_roots: vec![crates_root],
                    blocked_roots: vec![r"C:\Users".into(), r"C:\Windows".into()],
                },
                network: NetworkPolicy {
                    default_action: DefaultAction::Deny,
                    allowed_hosts: Vec::new(),
                    blocked_hosts: Vec::new(),
                },
                process: ProcessPolicy {
                    default_action: DefaultAction::Deny,
                    allowed_commands: vec!["git".into()],
                    blocked_commands: vec!["powershell".into(), "cmd".into()],
                },
            },
        },
    ]
}

fn finish(errors: ValidationErrors) -> Result<(), ValidationErrors> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_required(field: &str, value: &str, label: &str, errors: &mut ValidationErrors) {
    if value.trim().is_empty() {
        errors.push(
            ValidationErrorCode::MissingValue,
            field,
            format!("{label} must not be empty"),
        );
    }
}

fn validate_string_list(field: &str, values: &[String], errors: &mut ValidationErrors) {
    let mut seen = BTreeSet::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            errors.push(
                ValidationErrorCode::InvalidValue,
                field,
                "list values must not be empty",
            );
            continue;
        }

        if !seen.insert(trimmed.to_string()) {
            errors.push(
                ValidationErrorCode::DuplicateValue,
                field,
                format!("duplicate value `{trimmed}`"),
            );
        }
    }
}

fn validate_conflict(
    field: &str,
    allowed: &[String],
    blocked: &[String],
    message: &str,
    errors: &mut ValidationErrors,
) {
    let blocked_set: BTreeSet<_> = blocked.iter().map(|value| value.trim()).collect();
    if allowed
        .iter()
        .map(|value| value.trim())
        .any(|value| !value.is_empty() && blocked_set.contains(value))
    {
        errors.push(ValidationErrorCode::ConflictingValue, field, message);
    }
}

fn normalize_list(values: Vec<String>) -> Vec<String> {
    let mut normalized = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized
}

fn has_any(values: &[String]) -> bool {
    values.iter().any(|value| !value.trim().is_empty())
}

fn require_support(
    support: CapabilitySupport,
    field: &str,
    message: &str,
    errors: &mut ValidationErrors,
) {
    if matches!(support, CapabilitySupport::Unsupported) {
        errors.push(ValidationErrorCode::UnsupportedCapability, field, message);
    }
}
