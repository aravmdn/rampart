export type DefaultAction = "allow" | "deny";

export type SignatureStatus = "unsigned" | "valid" | "invalid";

export type ProfileDetail = {
  id: string;
  displayName: string;
  detail: string;
  filesystem: {
    readableRoots: string[];
    writableRoots: string[];
    blockedRoots: string[];
  };
  network: {
    defaultAction: DefaultAction;
    allowedHosts: string[];
    blockedHosts: string[];
  };
  process: {
    defaultAction: DefaultAction;
    allowedCommands: string[];
    blockedCommands: string[];
  };
  signatureStatus: SignatureStatus;
};

export const TASK3_API_NAMES = {
  loadLaunchContext: "load_launch_context",
  saveSelectedLaunchConfig: "save_selected_launch_config",
  launchSession: "launch_session",
  stopSession: "stop_session",
  streamSessionEvents: "stream_session_events",
  listSessionHistory: "list_session_history",
} as const;

export type PlatformKey = "windows" | "macos" | "linux";

export type CapabilityStatus = "supported" | "unsupported" | "partial";

export type CapabilityKey =
  | "filesystem_scope"
  | "network_egress"
  | "process_execution"
  | "violation_streaming"
  | "session_termination";

export type CapabilityItem = {
  key: CapabilityKey;
  status: CapabilityStatus;
  detail: string;
};

export type EngineCapabilitySnapshot = {
  engineName: string;
  platform: PlatformKey;
  capabilities: CapabilityItem[];
};

export type AgentTool = {
  id: string;
  label: string;
  detail: string;
  terminalFirst: boolean;
};

export type ProjectSummary = {
  id: string;
  label: string;
  path: string;
  source: string;
};

export type ProfileSummary = {
  id: string;
  displayName: string;
  detail: string;
  signatureStatus: SignatureStatus;
};

export type SessionStatus = "idle" | "launching" | "active" | "stopped" | "failed";

export type SessionState = {
  id: string | null;
  status: SessionStatus;
  profileId: string | null;
  agentId: string | null;
  projectPath: string | null;
};

export type AuditEventCategory = "session_lifecycle" | "policy_enforcement" | "system_alert";

export type AuditEvent = {
  id: string;
  kind:
    | "launch_succeeded"
    | "allow_observed"
    | "block_observed"
    | "session_stopped"
    | "filesystem_allowed"
    | "filesystem_blocked"
    | "network_allowed"
    | "network_blocked"
    | "process_allowed"
    | "process_blocked";
  category: AuditEventCategory;
  message: string;
};

export type ViolationExplanation = {
  ruleDescription: string;
  platformLimitation: {
    platform: string;
    engine: string;
    detail: string;
  } | null;
  remediationHint: string | null;
};

export type ViolationEvent = {
  id: string;
  operation: "read" | "write" | "execute" | "network";
  target: string;
  ruleId: string;
  ruleLabel: string;
  message: string;
  platformNote: string | null;
  explanation: ViolationExplanation | null;
};

export type IsolationMode = "windows-native" | "wsl2";

export type LaunchSessionRequest = {
  projectPath: string;
  agentId: string;
  profileId: string;
  isolationMode?: IsolationMode;
};

export type SelectedLaunchConfig = {
  projectPath: string | null;
  agentId: string | null;
  profileId: string | null;
};

export type PreflightSeverity = "pass" | "warning" | "fail";

export type PreflightDiagnostic = {
  severity: PreflightSeverity;
  label: string;
  detail: string;
};

export type PreflightReport = {
  ready: boolean;
  diagnostics: PreflightDiagnostic[];
};

export type LaunchContext = {
  projects: ProjectSummary[];
  agents: AgentTool[];
  profiles: ProfileSummary[];
  selected: SelectedLaunchConfig;
  capabilities: EngineCapabilitySnapshot;
};

export type SessionHistoryEntry = {
  session: SessionState & {
    startedAtMs: number;
    endedAtMs: number | null;
  };
  capabilitySnapshot: EngineCapabilitySnapshot | null;
  events: AuditEvent[];
  violations: ViolationEvent[];
};

export type SyncConfig = {
  endpointUrl: string;
  token: string;
  stripPaths: boolean;
};

export type SyncStatus = {
  configured: boolean;
  queueDepth: number;
  lastSyncAtMs: number | null;
  lastError: string | null;
};

export type DaemonApi = {
  loadLaunchContext: () => Promise<LaunchContext>;
  saveSelectedLaunchConfig: (selected: SelectedLaunchConfig) => Promise<void>;
  preflightCheck: (projectDir: string, agentId: string, profileId: string) => Promise<PreflightReport>;
  launchSession: (request: LaunchSessionRequest) => Promise<SessionState>;
  stopSession: (sessionId: string) => Promise<SessionState>;
  streamSessionEvents: (
    sessionId: string,
  ) => Promise<{ audit: AuditEvent[]; violations: ViolationEvent[] }>;
  listSessionHistory: () => Promise<SessionHistoryEntry[]>;
  loadProfile: (profileId: string) => Promise<ProfileDetail>;
  saveProfile: (profile: ProfileDetail) => Promise<void>;
  signProfile: (profileId: string, signingKeyB64: string) => Promise<void>;
  loadRemoteProfile: (url: string) => Promise<ProfileDetail>;
  configureSync: (config: SyncConfig) => Promise<void>;
  getSyncStatus: () => Promise<SyncStatus>;
  syncAuditEvents: () => Promise<SyncStatus>;
};
