import type {
  AgentTool,
  AuditEvent,
  DaemonApi,
  LaunchContext,
  EngineCapabilitySnapshot,
  LaunchSessionRequest,
  ProfileSummary,
  SelectedLaunchConfig,
  SessionHistoryEntry,
  SessionState,
  ViolationEvent,
} from "./contracts";

export const mockProjects = [
  { id: "project-rampart", label: "Rampart", path: "C:\\projects\\rampart" },
  { id: "project-docs", label: "Docs sandbox", path: "C:\\projects\\sandbox-docs" },
];

export const mockAgents: AgentTool[] = [
  {
    id: "codex",
    label: "Codex",
    detail: "OpenAI coding agent in local desktop flow.",
  },
  {
    id: "claude-code",
    label: "Claude Code",
    detail: "Anthropic agent with profile-driven launch.",
  },
];

const profiles: ProfileSummary[] = [
  {
    id: "windows-safe",
    displayName: "Windows Safe",
    detail: "Project scoped read/write. Network denied by default.",
  },
  {
    id: "windows-strict",
    displayName: "Windows Strict",
    detail: "Project read only outside src. Child process creation denied.",
  },
];

const capabilitySnapshot: EngineCapabilitySnapshot = {
  engineName: "rampart-windows-runtime-mock",
  platform: "windows",
  capabilities: [
    {
      key: "filesystem_scope",
      status: "supported",
      detail: "Scoped filesystem enforcement available in current mocked flow.",
    },
    {
      key: "process_execution",
      status: "supported",
      detail: "Child process policy contract exists in daemon and adapter stubs.",
    },
    {
      key: "network_egress",
      status: "partial",
      detail: "Network visibility partial in current Windows path. Full block not active yet.",
    },
    {
      key: "violation_streaming",
      status: "supported",
      detail: "Normalized block events stream into session view.",
    },
    {
      key: "session_termination",
      status: "supported",
      detail: "Session stop control reserved for daemon-owned process tracking.",
    },
  ],
};

const pause = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));
const sessions = new Map<string, SessionState>();

function buildAudit(sessionId: string): AuditEvent[] {
  return [
    {
      id: `${sessionId}-audit-launch`,
      kind: "launch_succeeded",
      message: "Session launched through mocked daemon contract.",
    },
    {
      id: `${sessionId}-audit-allow`,
      kind: "allow_observed",
      message: "Allowed read inside project scope: C:\\projects\\rampart\\README.md",
    },
    {
      id: `${sessionId}-audit-block`,
      kind: "block_observed",
      message: "Blocked read outside selected project roots.",
    },
  ];
}

function buildViolations(sessionId: string): ViolationEvent[] {
  return [
    {
      id: `${sessionId}-violation-1`,
      operation: "read",
      target: "C:\\Users\\dev\\.ssh\\config",
      ruleId: "project_scope",
      message: "Policy denied read outside allowed project roots.",
    },
  ];
}

export const mockDaemonClient: DaemonApi = {
  async loadLaunchContext(): Promise<LaunchContext> {
    await pause(80);
    return {
      projects: mockProjects.map((project) => ({
        ...project,
        source: "mock",
      })),
      agents: mockAgents,
      profiles,
      selected: {
        projectPath: mockProjects[0]?.path ?? null,
        agentId: mockAgents[0]?.id ?? null,
        profileId: profiles[0]?.id ?? null,
      },
      capabilities: capabilitySnapshot,
    };
  },
  async saveSelectedLaunchConfig(_selected: SelectedLaunchConfig) {
    await pause(20);
  },
  async launchSession(request: LaunchSessionRequest) {
    await pause(120);
    const id = `session-${sessions.size + 1}`;
    const session: SessionState = {
      id,
      status: "active",
      profileId: request.profileId,
      agentId: request.agentId,
      projectPath: request.projectPath,
    };
    sessions.set(id, session);
    return session;
  },
  async stopSession(sessionId: string) {
    await pause(80);
    const current = sessions.get(sessionId);
    const session: SessionState = {
      id: sessionId,
      status: "stopped",
      profileId: current?.profileId ?? null,
      agentId: current?.agentId ?? null,
      projectPath: current?.projectPath ?? null,
    };
    sessions.set(sessionId, session);
    return session;
  },
  async streamSessionEvents(sessionId: string) {
    await pause(80);
    return {
      audit: buildAudit(sessionId),
      violations: buildViolations(sessionId),
    };
  },
  async listSessionHistory(): Promise<SessionHistoryEntry[]> {
    await pause(40);
    return [];
  },
};
