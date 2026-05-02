import { invoke } from "@tauri-apps/api/core";
import type {
  AuditEvent,
  DaemonApi,
  LaunchContext,
  LaunchSessionRequest,
  OrgPolicy,
  OrgPolicyScope,
  PreflightReport,
  ProfileDetail,
  SelectedLaunchConfig,
  SessionHistoryEntry,
  SessionState,
  SyncConfig,
  SyncStatus,
} from "./contracts";

type RawLaunchContext = {
  projects: {
    id: string;
    label: string;
    path: string;
    source: string;
  }[];
  agents: {
    id: string;
    label: string;
    detail: string;
    terminal_first: boolean;
  }[];
  profiles: {
    id: string;
    display_name: string;
    detail: string;
    signature_status?: string;
  }[];
  selected: {
    project_path: string | null;
    agent_id: string | null;
    profile_id: string | null;
  };
  capabilities: {
    engine_name: string;
    platform: string;
  };
  capability_items: {
    key: string;
    status: string;
    detail: string;
  }[];
};

type RawViolation = {
  action: "read" | "write" | "execute" | "network";
  target: string;
  rule_id: string;
  rule_label: string;
  reason: string;
  platform_note: string | null;
  explanation: {
    rule_description: string;
    platform_limitation: {
      platform: string;
      engine: string;
      detail: string;
    } | null;
    remediation_hint: string | null;
  } | null;
};

type RawAuditEventInBatch = {
  kind: string;
  category: string;
  message: string;
  session_id: string;
  sequence: number;
  occurred_at_ms: number;
};

type RawHistoryEntry = {
  session: {
    id: string;
    status: string;
    profile_id: string;
    project_path: string;
    agent_tool?: RawAgentTool | null;
    started_at_ms: number;
    ended_at_ms: number | null;
  };
  capability_snapshot: {
    engine_name: string;
    platform: string;
    capability_items?: { key: string; status: string; detail: string }[];
  } | null;
  events: {
    kind: string;
    category: string;
    message: string;
    session_id: string;
    sequence: number;
    occurred_at_ms: number;
  }[];
  violations: RawViolation[];
};

type RawPreflightDiagnostic = {
  severity: string;
  label: string;
  detail: string;
  from_org_policy?: boolean;
};

type RawPreflightReport = {
  ready: boolean;
  diagnostics: RawPreflightDiagnostic[];
};

function mapPreflightReport(raw: RawPreflightReport): import("./contracts").PreflightReport {
  return {
    ready: raw.ready,
    diagnostics: raw.diagnostics.map((d) => ({
      severity: d.severity as import("./contracts").PreflightSeverity,
      label: d.label,
      detail: d.detail,
      fromOrgPolicy: d.from_org_policy ?? false,
    })),
  };
}

type RawOrgPolicyScope = {
  agent_types?: string[] | null;
  project_path_glob?: string | null;
};

type RawOrgPolicy = {
  id: string;
  name: string;
  description?: string | null;
  scope?: RawOrgPolicyScope | null;
  policy: {
    filesystem: {
      readable_roots: string[];
      writable_roots: string[];
      blocked_roots: string[];
    };
    network: {
      default_action: string;
      allowed_hosts: string[];
      blocked_hosts: string[];
    };
    process: {
      default_action: string;
      allowed_commands: string[];
      blocked_commands: string[];
    };
  };
  signature?: {
    signer: string;
    algorithm: string;
    value: string;
  } | null;
};

function mapOrgPolicy(raw: RawOrgPolicy): OrgPolicy {
  return {
    id: raw.id,
    name: raw.name,
    description: raw.description ?? null,
    scope: raw.scope
      ? {
          agentTypes: raw.scope.agent_types ?? null,
          projectPathGlob: raw.scope.project_path_glob ?? null,
        }
      : null,
    policy: {
      filesystem: {
        readableRoots: raw.policy.filesystem.readable_roots,
        writableRoots: raw.policy.filesystem.writable_roots,
        blockedRoots: raw.policy.filesystem.blocked_roots,
      },
      network: {
        defaultAction: raw.policy.network.default_action as import("./contracts").DefaultAction,
        allowedHosts: raw.policy.network.allowed_hosts,
        blockedHosts: raw.policy.network.blocked_hosts,
      },
      process: {
        defaultAction: raw.policy.process.default_action as import("./contracts").DefaultAction,
        allowedCommands: raw.policy.process.allowed_commands,
        blockedCommands: raw.policy.process.blocked_commands,
      },
    },
    signature: raw.signature ?? null,
  };
}

function mapLaunchContext(raw: RawLaunchContext): LaunchContext {
  return {
    projects: raw.projects,
    agents: raw.agents.map((agent) => ({
      id: agent.id,
      label: agent.label,
      detail: agent.detail,
      terminalFirst: agent.terminal_first,
    })),
    profiles: raw.profiles.map((profile) => ({
      id: profile.id,
      displayName: profile.display_name,
      detail: profile.detail,
      signatureStatus: (profile.signature_status ?? "unsigned") as import("./contracts").SignatureStatus,
    })),
    selected: {
      projectPath: raw.selected.project_path,
      agentId: raw.selected.agent_id,
      profileId: raw.selected.profile_id,
    },
    capabilities: {
      engineName: raw.capabilities.engine_name,
      platform: raw.capabilities.platform as "windows" | "macos" | "linux",
      capabilities: (raw.capability_items ?? []).map((item) => ({
        key: item.key as any,
        status: item.status as any,
        detail: item.detail,
      })),
    },
  };
}

type RawAgentTool = string | { Custom: { id: string; display_name?: string | null } };

function mapAgentTool(raw: RawAgentTool | undefined | null): string | null {
  if (!raw) return null;
  if (typeof raw === "string") {
    const map: Record<string, string> = {
      ClaudeCode: "claude-code",
      Codex: "codex",
      Cursor: "cursor",
      Copilot: "copilot",
      Aider: "aider",
      Goose: "goose",
      OpenCode: "opencode",
      GeminiCli: "gemini",
    };
    return map[raw] ?? raw.toLowerCase();
  }
  return raw.Custom?.id ?? null;
}

function mapSessionState(raw: {
  id: string;
  status: string;
  profile_id: string;
  project_path: string;
  agent_tool?: RawAgentTool | null;
  ended_at_ms?: number | null;
}): SessionState {
  const statusMap: Record<string, SessionState["status"]> = {
    pending: "launching",
    running: "active",
    finished: "stopped",
    failed: "failed",
    terminated: "stopped",
  };

  return {
    id: raw.id,
    status: statusMap[raw.status] ?? "failed",
    profileId: raw.profile_id,
    agentId: mapAgentTool(raw.agent_tool),
    projectPath: raw.project_path,
  };
}

function mapAuditKind(kind: string): AuditEvent["kind"] {
  const map: Record<string, AuditEvent["kind"]> = {
    "session-launched": "launch_succeeded",
    "session-ended": "session_stopped",
    "filesystem-allowed": "filesystem_allowed",
    "filesystem-blocked": "filesystem_blocked",
    "network-allowed": "network_allowed",
    "network-blocked": "network_blocked",
    "process-allowed": "process_allowed",
    "process-blocked": "process_blocked",
    "violation-recorded": "block_observed",
    "operation-observed": "allow_observed",
  };
  return map[kind] ?? "allow_observed";
}

function mapAuditCategory(category: string): AuditEvent["category"] {
  const map: Record<string, AuditEvent["category"]> = {
    "session-lifecycle": "session_lifecycle",
    "policy-enforcement": "policy_enforcement",
    "system-alert": "system_alert",
  };
  return map[category] ?? "policy_enforcement";
}

export const tauriDaemonClient: DaemonApi = {
  async loadLaunchContext() {
    const response = await invoke<RawLaunchContext>("load_launch_context");
    return mapLaunchContext(response);
  },
  async saveSelectedLaunchConfig(selected: SelectedLaunchConfig) {
    await invoke("save_selected_launch_config", { selected });
  },
  async preflightCheck(projectDir: string, agentId: string, profileId: string): Promise<PreflightReport> {
    const raw = await invoke<RawPreflightReport>("preflight_check", { projectDir, agentId, profileId });
    return mapPreflightReport(raw);
  },
  async launchSession(request: LaunchSessionRequest) {
    const response = await invoke<{
      id: string;
      status: string;
      profile_id: string;
      project_path: string;
    }>("launch_session", { request });
    return mapSessionState(response);
  },
  async stopSession(sessionId: string) {
    const response = await invoke<{
      id: string;
      status: string;
      profile_id: string;
      project_path: string;
    }>("stop_session", { sessionId });
    return mapSessionState(response);
  },
  async streamSessionEvents(sessionId: string) {
    const raw = await invoke<{ audit: RawAuditEventInBatch[]; violations: RawViolation[] }>(
      "stream_session_events",
      { sessionId },
    );
    return {
      audit: raw.audit.map((event, index) => ({
        id: `${sessionId}-event-${index}`,
        kind: mapAuditKind(event.kind),
        category: mapAuditCategory(event.category),
        message: event.message,
      })),
      violations: raw.violations.map((violation, index) => ({
        id: `${sessionId}-violation-${index}`,
        operation: violation.action,
        target: violation.target,
        ruleId: violation.rule_id,
        ruleLabel: violation.rule_label ?? "",
        message: violation.reason,
        platformNote: violation.platform_note ?? null,
        explanation: violation.explanation
          ? {
              ruleDescription: violation.explanation.rule_description,
              platformLimitation: violation.explanation.platform_limitation ?? null,
              remediationHint: violation.explanation.remediation_hint ?? null,
            }
          : null,
      })),
    };
  },
  async loadProfile(profileId: string): Promise<ProfileDetail> {
    const raw = await invoke<ProfileDetail & { signature_status?: string }>("load_profile", { profileId });
    return {
      ...raw,
      signatureStatus: (raw.signature_status ?? raw.signatureStatus ?? "unsigned") as import("./contracts").SignatureStatus,
    };
  },
  async saveProfile(profile: ProfileDetail): Promise<void> {
    await invoke("save_profile", { profile });
  },
  async signProfile(profileId: string, signingKeyB64: string): Promise<void> {
    await invoke("sign_profile", { profileId, signingKeyB64 });
  },
  async loadRemoteProfile(url: string): Promise<ProfileDetail> {
    const raw = await invoke<ProfileDetail & { signature_status?: string }>("load_remote_profile", { url });
    return {
      ...raw,
      signatureStatus: (raw.signature_status ?? raw.signatureStatus ?? "unsigned") as import("./contracts").SignatureStatus,
    };
  },
  async configureSync(config: SyncConfig): Promise<void> {
    await invoke("configure_sync", { config });
  },
  async getSyncStatus(): Promise<SyncStatus> {
    const raw = await invoke<{ configured: boolean; queue_depth: number; last_sync_at_ms?: number | null; last_error?: string | null }>("get_sync_status");
    return {
      configured: raw.configured,
      queueDepth: raw.queue_depth,
      lastSyncAtMs: raw.last_sync_at_ms ?? null,
      lastError: raw.last_error ?? null,
    };
  },
  async syncAuditEvents(): Promise<SyncStatus> {
    const raw = await invoke<{ configured: boolean; queue_depth: number; last_sync_at_ms?: number | null; last_error?: string | null }>("sync_audit_events");
    return {
      configured: raw.configured,
      queueDepth: raw.queue_depth,
      lastSyncAtMs: raw.last_sync_at_ms ?? null,
      lastError: raw.last_error ?? null,
    };
  },
  async configureOrgPolicyUrl(url: string | null): Promise<void> {
    await invoke("configure_org_policy_url", { url });
  },
  async fetchOrgPolicy(): Promise<import("./contracts").OrgPolicy | null> {
    const raw = await invoke<any>("fetch_org_policy");
    if (!raw) return null;
    return mapOrgPolicy(raw);
  },
  async currentOrgPolicy(): Promise<import("./contracts").OrgPolicy | null> {
    const raw = await invoke<any>("current_org_policy");
    if (!raw) return null;
    return mapOrgPolicy(raw);
  },
  async listSessionHistory() {
    const response = await invoke<RawHistoryEntry[]>("list_session_history");
    return response.map((entry) => ({
      session: {
        id: entry.session.id,
        status: mapSessionState(entry.session).status,
        profileId: entry.session.profile_id,
        agentId: mapAgentTool(entry.session.agent_tool),
        projectPath: entry.session.project_path,
        startedAtMs: entry.session.started_at_ms,
        endedAtMs: entry.session.ended_at_ms,
      },
      capabilitySnapshot: entry.capability_snapshot
        ? {
            engineName: entry.capability_snapshot.engine_name,
            platform: entry.capability_snapshot.platform as "windows" | "macos" | "linux",
            capabilities: (entry.capability_snapshot.capability_items ?? []).map((item) => ({
              key: item.key as any,
              status: item.status as any,
              detail: item.detail,
            })),
          }
        : null,
      events: entry.events.map((event, index) => ({
        id: `${entry.session.id}-event-${index}`,
        kind: mapAuditKind(event.kind),
        category: mapAuditCategory(event.category),
        message: event.message,
      })),
      violations: entry.violations.map((violation, index) => ({
        id: `${entry.session.id}-violation-${index}`,
        operation: violation.action,
        target: violation.target,
        ruleId: violation.rule_id,
        ruleLabel: violation.rule_label ?? "",
        message: violation.reason,
        platformNote: violation.platform_note ?? null,
        explanation: violation.explanation
          ? {
              ruleDescription: violation.explanation.rule_description,
              platformLimitation: violation.explanation.platform_limitation ?? null,
              remediationHint: violation.explanation.remediation_hint ?? null,
            }
          : null,
      })),
    }));
  },
};
