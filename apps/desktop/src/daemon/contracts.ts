export const TASK3_API_NAMES = {
  detectCapabilities: "DetectCapabilities",
  listProfiles: "ListProfiles",
  launchSession: "LaunchSession",
  stopSession: "StopSession",
  streamSessionEvents: "StreamSessionEvents",
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
};

export type ProfileSummary = {
  id: string;
  displayName: string;
  detail: string;
};

export type SessionStatus = "idle" | "launching" | "active" | "stopped" | "failed";

export type SessionState = {
  id: string | null;
  status: SessionStatus;
  profileId: string | null;
  agentId: string | null;
  projectPath: string | null;
};

export type AuditEvent = {
  id: string;
  kind: "launch_succeeded" | "allow_observed" | "block_observed" | "session_stopped";
  message: string;
};

export type ViolationEvent = {
  id: string;
  operation: "read" | "write" | "execute" | "network";
  target: string;
  ruleId: string;
  message: string;
};

export type LaunchSessionRequest = {
  projectPath: string;
  agentId: string;
  profileId: string;
};

export type DaemonApi = {
  detectCapabilities: () => Promise<EngineCapabilitySnapshot>;
  listProfiles: () => Promise<ProfileSummary[]>;
  launchSession: (request: LaunchSessionRequest) => Promise<SessionState>;
  stopSession: (sessionId: string) => Promise<SessionState>;
  streamSessionEvents: (
    sessionId: string,
  ) => Promise<{ audit: AuditEvent[]; violations: ViolationEvent[] }>;
};
