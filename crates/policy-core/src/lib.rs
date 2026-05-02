use base64::{engine::general_purpose, Engine as _};
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

/// An ed25519 signature block attached to a signed profile.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileSignature {
    /// Base64-encoded ed25519 verifying key (32 bytes).
    pub signer: String,
    /// Always "ed25519".
    pub algorithm: String,
    /// Base64-encoded ed25519 signature over the canonical profile bytes (64 bytes).
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SignatureStatus {
    #[default]
    Unsigned,
    Valid,
    Invalid,
}

#[derive(Debug, thiserror::Error)]
pub enum SignError {
    #[error("invalid signing key: expected 32-byte ed25519 seed")]
    InvalidKey,
    #[error("profile serialization failed")]
    SerializationFailed,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<ProfileSignature>,
}

/// Returns the canonical bytes to sign: the profile serialized to JSON with the
/// `signature` field absent. Deterministic given the same profile contents.
fn canonical_profile_bytes(profile: &Profile) -> Vec<u8> {
    let mut stripped = profile.clone();
    stripped.signature = None;
    serde_json::to_vec(&stripped).unwrap_or_default()
}

/// Verify the signature on a profile. Returns `Unsigned` if no signature block is
/// present, `Valid` if the signature is correct, `Invalid` otherwise.
pub fn verify_signature(profile: &Profile) -> SignatureStatus {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let sig_block = match &profile.signature {
        None => return SignatureStatus::Unsigned,
        Some(s) => s,
    };

    if sig_block.algorithm != "ed25519" {
        return SignatureStatus::Invalid;
    }

    let vk_bytes = match general_purpose::STANDARD.decode(&sig_block.signer) {
        Ok(b) => b,
        Err(_) => return SignatureStatus::Invalid,
    };
    let sig_bytes = match general_purpose::STANDARD.decode(&sig_block.value) {
        Ok(b) => b,
        Err(_) => return SignatureStatus::Invalid,
    };

    let vk_arr: [u8; 32] = match vk_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => return SignatureStatus::Invalid,
    };
    let vk = match VerifyingKey::from_bytes(&vk_arr) {
        Ok(k) => k,
        Err(_) => return SignatureStatus::Invalid,
    };

    let sig_arr: [u8; 64] = match sig_bytes.try_into() {
        Ok(arr) => arr,
        Err(_) => return SignatureStatus::Invalid,
    };
    let sig = Signature::from_bytes(&sig_arr);

    let message = canonical_profile_bytes(profile);
    match vk.verify(&message, &sig) {
        Ok(_) => SignatureStatus::Valid,
        Err(_) => SignatureStatus::Invalid,
    }
}

/// Sign a profile with an ed25519 private key seed (32 bytes). The `signer_label`
/// is not used for verification; the verifying key embedded in the signature block
/// is authoritative. Pass an empty string or a human-readable label as preferred.
pub fn sign_profile(
    profile: &mut Profile,
    _signer_label: &str,
    key_bytes: &[u8],
) -> Result<(), SignError> {
    use ed25519_dalek::{Signer, SigningKey};

    let key_arr: [u8; 32] = key_bytes.try_into().map_err(|_| SignError::InvalidKey)?;
    let signing_key = SigningKey::from_bytes(&key_arr);
    let verifying_key = signing_key.verifying_key();

    profile.signature = None;
    let message = canonical_profile_bytes(profile);
    if message.is_empty() {
        return Err(SignError::SerializationFailed);
    }

    let signature = signing_key.sign(&message);
    profile.signature = Some(ProfileSignature {
        signer: general_purpose::STANDARD.encode(verifying_key.as_bytes()),
        algorithm: "ed25519".into(),
        value: general_purpose::STANDARD.encode(signature.to_bytes()),
    });

    Ok(())
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct OrgPolicyScope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_types: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_path_glob: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct OrgPolicy {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<OrgPolicyScope>,
    pub policy: Policy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<ProfileSignature>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "kebab-case")]
pub enum IsolationMode {
    /// Windows-native enforcement: Job Objects + WFP + Low Integrity token + SACL.
    #[default]
    WindowsNative,
    /// Run the agent inside WSL2 for Linux-native enforcement (Landlock + seccomp).
    /// Only meaningful on Windows hosts with WSL2 installed.
    Wsl2,
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
    #[serde(default)]
    pub isolation_mode: IsolationMode,
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
        AgentTool::Cursor => {
            let cursor_config = format!("{user_profile}\\.cursor");
            vec![
                Profile {
                    id: "cursor.standard".into(),
                    name: "Cursor Standard".into(),
                    description: Some(
                        "Project read/write, Cursor config readable, Cursor/Anthropic/OpenAI APIs allowed.".into(),
                    ),
                    extends: None,
                    policy: Policy {
                        filesystem: FilesystemPolicy {
                            readable_roots: vec![normalized_root.clone(), cursor_config.clone()],
                            writable_roots: vec![normalized_root.clone()],
                            blocked_roots: vec![],
                        },
                        network: NetworkPolicy {
                            default_action: DefaultAction::Deny,
                            allowed_hosts: vec![
                                "cursor.sh".into(),
                                "api2.cursor.sh".into(),
                                "api.anthropic.com".into(),
                                "api.openai.com".into(),
                            ],
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
                    id: "cursor.strict".into(),
                    name: "Cursor Strict".into(),
                    description: Some(
                        "Project read/write only. Network denied. Only git permitted.".into(),
                    ),
                    extends: None,
                    policy: Policy {
                        filesystem: FilesystemPolicy {
                            readable_roots: vec![normalized_root.clone(), cursor_config],
                            writable_roots: vec![normalized_root.clone()],
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
        AgentTool::Copilot => {
            vec![
                Profile {
                    id: "copilot.standard".into(),
                    name: "GitHub Copilot Standard".into(),
                    description: Some(
                        "Project read/write, GitHub and OpenAI APIs allowed. git and node permitted.".into(),
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
                                "api.github.com".into(),
                                "copilot-proxy.githubusercontent.com".into(),
                                "api.openai.com".into(),
                            ],
                            blocked_hosts: vec![],
                        },
                        process: ProcessPolicy {
                            default_action: DefaultAction::Deny,
                            allowed_commands: vec![
                                "git".into(),
                                "node".into(),
                                "npm".into(),
                            ],
                            blocked_commands: vec!["powershell".into()],
                        },
                    },
                },
                Profile {
                    id: "copilot.strict".into(),
                    name: "GitHub Copilot Strict".into(),
                    description: Some(
                        "Project read/write only. Network denied. Only git permitted.".into(),
                    ),
                    extends: None,
                    policy: Policy {
                        filesystem: FilesystemPolicy {
                            readable_roots: vec![normalized_root.clone()],
                            writable_roots: vec![normalized_root.clone()],
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
        AgentTool::Goose => {
            let goose_config = format!("{user_profile}\\.config\\goose");
            vec![
                Profile {
                    id: "goose.standard".into(),
                    name: "Goose Standard".into(),
                    description: Some(
                        "Project read/write, Goose config readable, Anthropic and OpenAI APIs allowed.".into(),
                    ),
                    extends: None,
                    policy: Policy {
                        filesystem: FilesystemPolicy {
                            readable_roots: vec![normalized_root.clone(), goose_config.clone()],
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
                                "node".into(),
                                "npm".into(),
                            ],
                            blocked_commands: vec!["powershell".into()],
                        },
                    },
                },
                Profile {
                    id: "goose.strict".into(),
                    name: "Goose Strict".into(),
                    description: Some(
                        "Project read/write only. Network denied. Only git permitted.".into(),
                    ),
                    extends: None,
                    policy: Policy {
                        filesystem: FilesystemPolicy {
                            readable_roots: vec![normalized_root.clone(), goose_config],
                            writable_roots: vec![normalized_root.clone()],
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
        AgentTool::OpenCode => {
            vec![
                Profile {
                    id: "opencode.standard".into(),
                    name: "OpenCode Standard".into(),
                    description: Some(
                        "Project read/write, Anthropic and OpenAI APIs allowed. git and node permitted.".into(),
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
                                "node".into(),
                                "npm".into(),
                                "pnpm".into(),
                            ],
                            blocked_commands: vec!["powershell".into()],
                        },
                    },
                },
                Profile {
                    id: "opencode.strict".into(),
                    name: "OpenCode Strict".into(),
                    description: Some(
                        "Project read/write only. Network denied. Only git permitted.".into(),
                    ),
                    extends: None,
                    policy: Policy {
                        filesystem: FilesystemPolicy {
                            readable_roots: vec![normalized_root.clone()],
                            writable_roots: vec![normalized_root.clone()],
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
        AgentTool::GeminiCli => {
            let gemini_config = format!("{user_profile}\\.gemini");
            vec![
                Profile {
                    id: "gemini.standard".into(),
                    name: "Gemini CLI Standard".into(),
                    description: Some(
                        "Project read/write, Gemini config readable, Google Generative AI API allowed.".into(),
                    ),
                    extends: None,
                    policy: Policy {
                        filesystem: FilesystemPolicy {
                            readable_roots: vec![normalized_root.clone(), gemini_config.clone()],
                            writable_roots: vec![normalized_root.clone()],
                            blocked_roots: vec![],
                        },
                        network: NetworkPolicy {
                            default_action: DefaultAction::Deny,
                            allowed_hosts: vec![
                                "generativelanguage.googleapis.com".into(),
                                "aiplatform.googleapis.com".into(),
                            ],
                            blocked_hosts: vec![],
                        },
                        process: ProcessPolicy {
                            default_action: DefaultAction::Deny,
                            allowed_commands: vec![
                                "git".into(),
                                "node".into(),
                                "npm".into(),
                            ],
                            blocked_commands: vec!["powershell".into()],
                        },
                    },
                },
                Profile {
                    id: "gemini.strict".into(),
                    name: "Gemini CLI Strict".into(),
                    description: Some(
                        "Project read/write only. Network denied. Only git permitted.".into(),
                    ),
                    extends: None,
                    policy: Policy {
                        filesystem: FilesystemPolicy {
                            readable_roots: vec![normalized_root.clone(), gemini_config],
                            writable_roots: vec![normalized_root.clone()],
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

/// Decide whether an `OrgPolicy` applies to a given (agent_type, project_path) pair.
///
/// - If `org.scope` is None → applies to everything → return true.
/// - If `scope.agent_types` is Some, the agent_type must be in the list (case-sensitive).
///   If None → no agent restriction.
/// - If `scope.project_path_glob` is Some, match using fnmatch-style glob: `*` (any chars
///   except path separator), `**` (any chars including separators), `?` (single char).
///   Path separators are `/` and `\` (both treated as separators). Comparison is
///   case-insensitive on Windows for simplicity.
/// - All present scope fields must match (AND logic).
pub fn org_policy_applies(org: &OrgPolicy, agent_type: &str, project_path: &str) -> bool {
    let scope = match &org.scope {
        None => return true,
        Some(s) => s,
    };

    if let Some(agent_types) = &scope.agent_types {
        if !agent_types.contains(&agent_type.to_string()) {
            return false;
        }
    }

    if let Some(glob) = &scope.project_path_glob {
        if !glob_match(glob, project_path) {
            return false;
        }
    }

    true
}

/// Simple fnmatch-style glob matching supporting `*`, `**`, and `?`.
/// - `*` matches any characters except path separators (`/` and `\`)
/// - `**` matches any characters including path separators
/// - `?` matches exactly one character
/// - Comparison is case-insensitive (both pattern and input are lowercased)
fn glob_match(pattern: &str, input: &str) -> bool {
    glob_match_recursive(pattern, input, 0, 0)
}

fn glob_match_recursive(pattern: &str, input: &str, p_idx: usize, i_idx: usize) -> bool {
    let pattern_chars: Vec<char> = pattern.to_lowercase().chars().collect();
    let input_chars: Vec<char> = input.to_lowercase().chars().collect();

    if p_idx == pattern_chars.len() {
        return i_idx == input_chars.len();
    }

    if p_idx < pattern_chars.len() && pattern_chars[p_idx] == '*' {
        if p_idx + 1 < pattern_chars.len() && pattern_chars[p_idx + 1] == '*' {
            for j in i_idx..=input_chars.len() {
                if glob_match_recursive(pattern, input, p_idx + 2, j) {
                    return true;
                }
            }
            return false;
        } else {
            for j in i_idx..=input_chars.len() {
                if j < input_chars.len() && (input_chars[j] == '/' || input_chars[j] == '\\') {
                    break;
                }
                if glob_match_recursive(pattern, input, p_idx + 1, j) {
                    return true;
                }
            }
            return false;
        }
    }

    if i_idx >= input_chars.len() {
        return false;
    }

    if pattern_chars[p_idx] == '?' {
        return glob_match_recursive(pattern, input, p_idx + 1, i_idx + 1);
    }

    if pattern_chars[p_idx] == input_chars[i_idx] {
        return glob_match_recursive(pattern, input, p_idx + 1, i_idx + 1);
    }

    false
}

/// Merge `local` and `org` into a single `Profile` applying the most-restrictive-wins rule.
///
/// - `DefaultAction`: `Deny` dominates `Allow` for both network and process.
/// - Allow-lists (`readable_roots`, `writable_roots`, `allowed_hosts`, `allowed_commands`):
///   intersection — org floor narrows local, never widens.
/// - Deny-lists (`blocked_roots`, `blocked_hosts`, `blocked_commands`):
///   union — either side blocking makes it blocked.
///
/// The returned `Profile` clones `id`, `name`, `description`, `extends`, and `signature`
/// from `local`. `scope` on `OrgPolicy` is not consulted here; the caller is responsible
/// for determining that the org policy applies before calling this function.
pub fn resolve_effective_policy(local: &Profile, org: &OrgPolicy) -> Profile {
    let merged_fs = merge_filesystem(&local.policy.filesystem, &org.policy.filesystem);
    let merged_net = merge_network(&local.policy.network, &org.policy.network);
    let merged_proc = merge_process(&local.policy.process, &org.policy.process);

    Profile {
        id: local.id.clone(),
        name: local.name.clone(),
        description: local.description.clone(),
        extends: local.extends.clone(),
        signature: local.signature.clone(),
        policy: Policy {
            filesystem: merged_fs,
            network: merged_net,
            process: merged_proc,
        },
    }
}

fn merge_filesystem(local: &FilesystemPolicy, org: &FilesystemPolicy) -> FilesystemPolicy {
    FilesystemPolicy {
        readable_roots: intersect_lists(&local.readable_roots, &org.readable_roots),
        writable_roots: intersect_lists(&local.writable_roots, &org.writable_roots),
        blocked_roots: union_lists(&local.blocked_roots, &org.blocked_roots),
    }
}

fn merge_network(local: &NetworkPolicy, org: &NetworkPolicy) -> NetworkPolicy {
    NetworkPolicy {
        default_action: stricter_default(local.default_action, org.default_action),
        allowed_hosts: intersect_lists(&local.allowed_hosts, &org.allowed_hosts),
        blocked_hosts: union_lists(&local.blocked_hosts, &org.blocked_hosts),
    }
}

fn merge_process(local: &ProcessPolicy, org: &ProcessPolicy) -> ProcessPolicy {
    ProcessPolicy {
        default_action: stricter_default(local.default_action, org.default_action),
        allowed_commands: intersect_lists(&local.allowed_commands, &org.allowed_commands),
        blocked_commands: union_lists(&local.blocked_commands, &org.blocked_commands),
    }
}

/// `Deny` strictly dominates `Allow`. Returns `Deny` if either side is `Deny`.
fn stricter_default(a: DefaultAction, b: DefaultAction) -> DefaultAction {
    if matches!(a, DefaultAction::Deny) || matches!(b, DefaultAction::Deny) {
        DefaultAction::Deny
    } else {
        DefaultAction::Allow
    }
}

/// Intersection of two allow-lists. If either list is empty the intersection is empty,
/// because an empty org allow-list means "nothing is permitted" — the org floor allows
/// nothing and that is strictly more restrictive than any non-empty local allow-list.
///
/// Exception: if the org list is empty AND the local list is non-empty, the org has not
/// configured any restriction on this dimension (it is absent/unconstrained). In that
/// case we preserve the local list. An org "floor" only constrains when it actually
/// specifies entries.
///
/// Concretely:
/// - org empty, local non-empty → local (org has no opinion; leave local alone)
/// - org non-empty, local empty → empty (org allows some things; local allows none — intersection is ∅)
/// - both non-empty → set intersection
/// - both empty → empty
fn intersect_lists(local: &[String], org: &[String]) -> Vec<String> {
    if org.is_empty() {
        return local.to_vec();
    }
    let org_set: BTreeSet<&str> = org.iter().map(|s| s.as_str()).collect();
    local
        .iter()
        .filter(|s| org_set.contains(s.as_str()))
        .cloned()
        .collect()
}

/// Union of two deny-lists. Preserves insertion order of local first, then any org
/// entries not already present.
fn union_lists(local: &[String], org: &[String]) -> Vec<String> {
    let mut result = local.to_vec();
    let local_set: BTreeSet<&str> = local.iter().map(|s| s.as_str()).collect();
    for entry in org {
        if !local_set.contains(entry.as_str()) {
            result.push(entry.clone());
        }
    }
    result
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

#[cfg(test)]
mod merge_tests {
    use super::*;

    fn make_org(policy: Policy) -> OrgPolicy {
        OrgPolicy {
            id: "org-floor".into(),
            name: "Org Floor".into(),
            description: None,
            scope: None,
            policy,
            signature: None,
        }
    }

    fn make_local(policy: Policy) -> Profile {
        Profile {
            id: "local-profile".into(),
            name: "Local Profile".into(),
            description: None,
            extends: None,
            policy,
            signature: None,
        }
    }

    #[test]
    fn org_deny_default_dominates_local_allow() {
        let local = make_local(Policy {
            network: NetworkPolicy {
                default_action: DefaultAction::Allow,
                allowed_hosts: vec![],
                blocked_hosts: vec![],
            },
            process: ProcessPolicy {
                default_action: DefaultAction::Allow,
                allowed_commands: vec![],
                blocked_commands: vec![],
            },
            filesystem: FilesystemPolicy::default(),
        });
        let org = make_org(Policy {
            network: NetworkPolicy {
                default_action: DefaultAction::Deny,
                allowed_hosts: vec![],
                blocked_hosts: vec![],
            },
            process: ProcessPolicy {
                default_action: DefaultAction::Deny,
                allowed_commands: vec![],
                blocked_commands: vec![],
            },
            filesystem: FilesystemPolicy::default(),
        });
        let merged = resolve_effective_policy(&local, &org);
        assert_eq!(merged.policy.network.default_action, DefaultAction::Deny);
        assert_eq!(merged.policy.process.default_action, DefaultAction::Deny);
    }

    #[test]
    fn allow_list_intersection_narrows_correctly() {
        let local = make_local(Policy {
            network: NetworkPolicy {
                default_action: DefaultAction::Deny,
                allowed_hosts: vec!["api.example.com".into(), "cdn.example.com".into()],
                blocked_hosts: vec![],
            },
            process: ProcessPolicy {
                default_action: DefaultAction::Deny,
                allowed_commands: vec!["git".into(), "node".into(), "npm".into()],
                blocked_commands: vec![],
            },
            filesystem: FilesystemPolicy {
                readable_roots: vec![r"C:\project".into(), r"C:\home".into()],
                writable_roots: vec![r"C:\project".into(), r"C:\tmp".into()],
                blocked_roots: vec![],
            },
        });
        let org = make_org(Policy {
            network: NetworkPolicy {
                default_action: DefaultAction::Deny,
                allowed_hosts: vec!["api.example.com".into()],
                blocked_hosts: vec![],
            },
            process: ProcessPolicy {
                default_action: DefaultAction::Deny,
                allowed_commands: vec!["git".into(), "node".into()],
                blocked_commands: vec![],
            },
            filesystem: FilesystemPolicy {
                readable_roots: vec![r"C:\project".into()],
                writable_roots: vec![r"C:\project".into()],
                blocked_roots: vec![],
            },
        });
        let merged = resolve_effective_policy(&local, &org);
        assert_eq!(merged.policy.network.allowed_hosts, vec!["api.example.com"]);
        assert_eq!(
            merged.policy.process.allowed_commands,
            vec!["git".to_string(), "node".to_string()]
        );
        assert_eq!(merged.policy.filesystem.readable_roots, vec![r"C:\project"]);
        assert_eq!(merged.policy.filesystem.writable_roots, vec![r"C:\project"]);
    }

    #[test]
    fn deny_list_union_broadens_correctly() {
        let local = make_local(Policy {
            network: NetworkPolicy {
                default_action: DefaultAction::Deny,
                allowed_hosts: vec![],
                blocked_hosts: vec!["malware.example.com".into()],
            },
            process: ProcessPolicy {
                default_action: DefaultAction::Deny,
                allowed_commands: vec![],
                blocked_commands: vec!["powershell".into()],
            },
            filesystem: FilesystemPolicy {
                readable_roots: vec![],
                writable_roots: vec![],
                blocked_roots: vec![r"C:\Users".into()],
            },
        });
        let org = make_org(Policy {
            network: NetworkPolicy {
                default_action: DefaultAction::Deny,
                allowed_hosts: vec![],
                blocked_hosts: vec!["phishing.example.com".into()],
            },
            process: ProcessPolicy {
                default_action: DefaultAction::Deny,
                allowed_commands: vec![],
                blocked_commands: vec!["cmd".into()],
            },
            filesystem: FilesystemPolicy {
                readable_roots: vec![],
                writable_roots: vec![],
                blocked_roots: vec![r"C:\Windows".into()],
            },
        });
        let merged = resolve_effective_policy(&local, &org);
        assert!(merged
            .policy
            .network
            .blocked_hosts
            .contains(&"malware.example.com".to_string()));
        assert!(merged
            .policy
            .network
            .blocked_hosts
            .contains(&"phishing.example.com".to_string()));
        assert!(merged
            .policy
            .process
            .blocked_commands
            .contains(&"powershell".to_string()));
        assert!(merged
            .policy
            .process
            .blocked_commands
            .contains(&"cmd".to_string()));
        assert!(merged
            .policy
            .filesystem
            .blocked_roots
            .contains(&r"C:\Users".to_string()));
        assert!(merged
            .policy
            .filesystem
            .blocked_roots
            .contains(&r"C:\Windows".to_string()));
    }

    #[test]
    fn empty_org_policy_leaves_local_unchanged() {
        let local_policy = Policy {
            network: NetworkPolicy {
                default_action: DefaultAction::Allow,
                allowed_hosts: vec!["api.example.com".into()],
                blocked_hosts: vec!["bad.example.com".into()],
            },
            process: ProcessPolicy {
                default_action: DefaultAction::Allow,
                allowed_commands: vec!["git".into()],
                blocked_commands: vec!["powershell".into()],
            },
            filesystem: FilesystemPolicy {
                readable_roots: vec![r"C:\project".into()],
                writable_roots: vec![r"C:\project\out".into()],
                blocked_roots: vec![r"C:\secret".into()],
            },
        };
        let local = make_local(local_policy.clone());
        let org = make_org(Policy::default());
        let merged = resolve_effective_policy(&local, &org);
        assert_eq!(merged.policy, local_policy);
        assert_eq!(merged.id, "local-profile");
        assert_eq!(merged.name, "Local Profile");
    }

    #[test]
    fn metadata_cloned_from_local() {
        let local = Profile {
            id: "my-id".into(),
            name: "My Name".into(),
            description: Some("desc".into()),
            extends: Some("parent".into()),
            policy: Policy::default(),
            signature: None,
        };
        let org = make_org(Policy::default());
        let merged = resolve_effective_policy(&local, &org);
        assert_eq!(merged.id, "my-id");
        assert_eq!(merged.name, "My Name");
        assert_eq!(merged.description.as_deref(), Some("desc"));
        assert_eq!(merged.extends.as_deref(), Some("parent"));
    }
}

#[cfg(test)]
mod org_scope_tests {
    use super::*;

    fn make_org_with_scope(scope: Option<OrgPolicyScope>) -> OrgPolicy {
        OrgPolicy {
            id: "org-policy".into(),
            name: "Org Policy".into(),
            description: None,
            scope,
            policy: Policy::default(),
            signature: None,
        }
    }

    #[test]
    fn scope_none_applies_to_everything() {
        let org = make_org_with_scope(None);
        assert!(org_policy_applies(&org, "claude-code", "C:\\project\\foo"));
        assert!(org_policy_applies(&org, "codex", "/home/user/project"));
        assert!(org_policy_applies(&org, "any-agent", ""));
    }

    #[test]
    fn agent_types_restriction_matches() {
        let scope = OrgPolicyScope {
            agent_types: Some(vec!["claude-code".into(), "aider".into()]),
            project_path_glob: None,
        };
        let org = make_org_with_scope(Some(scope));

        assert!(org_policy_applies(&org, "claude-code", "C:\\project"));
        assert!(org_policy_applies(&org, "aider", "C:\\project"));
        assert!(!org_policy_applies(&org, "codex", "C:\\project"));
        assert!(!org_policy_applies(&org, "Claude-Code", "C:\\project"));
    }

    #[test]
    fn agent_types_none_means_no_restriction() {
        let scope = OrgPolicyScope {
            agent_types: None,
            project_path_glob: Some("**".into()),
        };
        let org = make_org_with_scope(Some(scope));

        assert!(org_policy_applies(&org, "claude-code", "C:\\project"));
        assert!(org_policy_applies(&org, "codex", "C:\\project"));
        assert!(org_policy_applies(&org, "any-agent", "C:\\project"));
    }

    #[test]
    fn glob_star_matches_except_path_separators() {
        let scope = OrgPolicyScope {
            agent_types: None,
            project_path_glob: Some("*.rs".into()),
        };
        let org = make_org_with_scope(Some(scope));

        assert!(org_policy_applies(&org, "any", "file.rs"));
        assert!(org_policy_applies(&org, "any", "MAIN.RS"));
        assert!(!org_policy_applies(&org, "any", "src/main.rs"));
        assert!(!org_policy_applies(&org, "any", "src\\main.rs"));
    }

    #[test]
    fn glob_double_star_matches_including_path_separators() {
        let scope = OrgPolicyScope {
            agent_types: None,
            project_path_glob: Some("**/foo/**".into()),
        };
        let org = make_org_with_scope(Some(scope));

        assert!(org_policy_applies(&org, "any", "C:\\work\\foo\\bar"));
        assert!(org_policy_applies(&org, "any", "c:\\work\\foo\\bar"));
        assert!(org_policy_applies(&org, "any", "/home/work/foo/bar"));
        assert!(org_policy_applies(&org, "any", "/foo/bar"));
        assert!(org_policy_applies(&org, "any", "foo/bar"));
        assert!(!org_policy_applies(&org, "any", "C:\\work\\foobar"));
    }

    #[test]
    fn glob_question_mark_single_char() {
        let scope = OrgPolicyScope {
            agent_types: None,
            project_path_glob: Some("test?.rs".into()),
        };
        let org = make_org_with_scope(Some(scope));

        assert!(org_policy_applies(&org, "any", "test1.rs"));
        assert!(org_policy_applies(&org, "any", "testa.rs"));
        assert!(!org_policy_applies(&org, "any", "test.rs"));
        assert!(!org_policy_applies(&org, "any", "test12.rs"));
    }

    #[test]
    fn combined_scope_both_must_match() {
        let scope = OrgPolicyScope {
            agent_types: Some(vec!["claude-code".into()]),
            project_path_glob: Some("**/secure/**".into()),
        };
        let org = make_org_with_scope(Some(scope));

        assert!(org_policy_applies(&org, "claude-code", "C:\\secure\\project"));
        assert!(!org_policy_applies(&org, "codex", "C:\\secure\\project"));
        assert!(!org_policy_applies(&org, "claude-code", "C:\\public\\project"));
    }

    #[test]
    fn glob_case_insensitive_matching() {
        let scope = OrgPolicyScope {
            agent_types: None,
            project_path_glob: Some("**/FOO/**".into()),
        };
        let org = make_org_with_scope(Some(scope));

        assert!(org_policy_applies(&org, "any", "C:\\Work\\foo\\bar"));
        assert!(org_policy_applies(&org, "any", "C:\\WORK\\FOO\\BAR"));
        assert!(org_policy_applies(&org, "any", "/work/FoO/bar"));
    }
}

#[cfg(test)]
mod preset_tests {
    use super::*;

    fn presets(tool: AgentTool) -> Vec<Profile> {
        agent_profile_presets(&tool, "C:\\projects\\myapp")
    }

    #[test]
    fn cursor_presets_have_correct_ids() {
        let ps = presets(AgentTool::Cursor);
        assert_eq!(ps.len(), 2);
        assert_eq!(ps[0].id, "cursor.standard");
        assert_eq!(ps[1].id, "cursor.strict");
    }

    #[test]
    fn cursor_standard_allows_cursor_api() {
        let ps = presets(AgentTool::Cursor);
        let std = &ps[0];
        assert!(std.policy.network.allowed_hosts.iter().any(|h| h.contains("cursor.sh")));
        assert_eq!(std.policy.network.default_action, DefaultAction::Deny);
    }

    #[test]
    fn cursor_strict_denies_network() {
        let ps = presets(AgentTool::Cursor);
        let strict = &ps[1];
        assert!(strict.policy.network.allowed_hosts.is_empty());
    }

    #[test]
    fn copilot_presets_have_correct_ids() {
        let ps = presets(AgentTool::Copilot);
        assert_eq!(ps.len(), 2);
        assert_eq!(ps[0].id, "copilot.standard");
        assert_eq!(ps[1].id, "copilot.strict");
    }

    #[test]
    fn copilot_standard_allows_github_api() {
        let ps = presets(AgentTool::Copilot);
        let std = &ps[0];
        assert!(std.policy.network.allowed_hosts.iter().any(|h| h.contains("github.com")));
    }

    #[test]
    fn goose_presets_have_correct_ids() {
        let ps = presets(AgentTool::Goose);
        assert_eq!(ps.len(), 2);
        assert_eq!(ps[0].id, "goose.standard");
        assert_eq!(ps[1].id, "goose.strict");
    }

    #[test]
    fn goose_standard_allows_anthropic_and_openai() {
        let ps = presets(AgentTool::Goose);
        let std = &ps[0];
        assert!(std.policy.network.allowed_hosts.iter().any(|h| h == "api.anthropic.com"));
        assert!(std.policy.network.allowed_hosts.iter().any(|h| h == "api.openai.com"));
    }

    #[test]
    fn opencode_presets_have_correct_ids() {
        let ps = presets(AgentTool::OpenCode);
        assert_eq!(ps.len(), 2);
        assert_eq!(ps[0].id, "opencode.standard");
        assert_eq!(ps[1].id, "opencode.strict");
    }

    #[test]
    fn gemini_presets_have_correct_ids() {
        let ps = presets(AgentTool::GeminiCli);
        assert_eq!(ps.len(), 2);
        assert_eq!(ps[0].id, "gemini.standard");
        assert_eq!(ps[1].id, "gemini.strict");
    }

    #[test]
    fn gemini_standard_allows_google_api() {
        let ps = presets(AgentTool::GeminiCli);
        let std = &ps[0];
        assert!(std.policy.network.allowed_hosts.iter().any(|h| h.contains("googleapis.com")));
    }

    #[test]
    fn gemini_strict_denies_network() {
        let ps = presets(AgentTool::GeminiCli);
        let strict = &ps[1];
        assert!(strict.policy.network.allowed_hosts.is_empty());
        assert_eq!(strict.policy.network.default_action, DefaultAction::Deny);
    }

    #[test]
    fn all_standard_presets_include_project_root_in_writable() {
        let root = "C:\\projects\\myapp";
        let tools = vec![
            AgentTool::Cursor,
            AgentTool::Copilot,
            AgentTool::Goose,
            AgentTool::OpenCode,
            AgentTool::GeminiCli,
        ];
        for tool in tools {
            let ps = agent_profile_presets(&tool, root);
            let std = &ps[0];
            assert!(
                std.policy.filesystem.writable_roots.iter().any(|r| r == root),
                "{:?} standard preset missing project root in writable_roots",
                std.id
            );
        }
    }

    #[test]
    fn all_standard_presets_deny_network_by_default() {
        let tools = vec![
            AgentTool::Cursor,
            AgentTool::Copilot,
            AgentTool::Goose,
            AgentTool::OpenCode,
            AgentTool::GeminiCli,
        ];
        for tool in tools {
            let ps = presets(tool);
            let std = &ps[0];
            assert_eq!(
                std.policy.network.default_action,
                DefaultAction::Deny,
                "{} standard preset should deny network by default",
                std.id
            );
        }
    }
}
