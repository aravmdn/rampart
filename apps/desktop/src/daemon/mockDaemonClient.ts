import type {
  AgentTool,
  AuditEvent,
  DaemonApi,
  DefaultAction,
  LaunchContext,
  EngineCapabilitySnapshot,
  LaunchSessionRequest,
  PreflightReport,
  ProfileDetail,
  ProfileSummary,
  SelectedLaunchConfig,
  SessionHistoryEntry,
  SessionState,
  SignatureStatus,
  ViolationEvent,
  ViolationExplanation,
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
    terminalFirst: true,
  },
  {
    id: "claude-code",
    label: "Claude Code",
    detail: "Anthropic agent with profile-driven launch.",
    terminalFirst: true,
  },
];

const allProfilePresets: Record<string, ProfileSummary[]> = {
  "claude-code": [
    {
      id: "claude-code.standard",
      displayName: "Claude Code Standard",
      detail: "Project read/write, Claude config readable, Anthropic API allowed.",
      signatureStatus: "unsigned" as SignatureStatus,
    },
    {
      id: "claude-code.strict",
      displayName: "Claude Code Strict",
      detail: "Project read/write only. Network denied. Child processes denied except git.",
      signatureStatus: "unsigned" as SignatureStatus,
    },
  ],
  codex: [
    {
      id: "codex.standard",
      displayName: "Codex Standard",
      detail: "Project read/write, OpenAI API allowed. git, node, npm permitted.",
      signatureStatus: "unsigned" as SignatureStatus,
    },
    {
      id: "codex.strict",
      displayName: "Codex Strict",
      detail: "Project read/write only. Network denied. Only git permitted.",
      signatureStatus: "unsigned" as SignatureStatus,
    },
  ],
};

const genericProfiles: ProfileSummary[] = [
  {
    id: "windows-safe",
    displayName: "Windows Safe",
    detail: "Project scoped read/write. Network denied by default.",
    signatureStatus: "unsigned" as SignatureStatus,
  },
  {
    id: "windows-strict",
    displayName: "Windows Strict",
    detail: "Project read only outside src. Child process creation denied.",
    signatureStatus: "unsigned" as SignatureStatus,
  },
];

const mockProfileDetails: Record<string, ProfileDetail> = {
  "claude-code.standard": {
    id: "claude-code.standard",
    displayName: "Claude Code Standard",
    detail: "Project read/write, Claude config readable, Anthropic API allowed.",
    signatureStatus: "unsigned" as SignatureStatus,
    filesystem: {
      readableRoots: ["C:\\projects\\rampart", "C:\\Users\\user\\.claude"],
      writableRoots: ["C:\\projects\\rampart"],
      blockedRoots: [],
    },
    network: {
      defaultAction: "deny" as DefaultAction,
      allowedHosts: ["api.anthropic.com"],
      blockedHosts: [],
    },
    process: {
      defaultAction: "deny" as DefaultAction,
      allowedCommands: ["git", "node", "npm", "pnpm"],
      blockedCommands: ["powershell"],
    },
  },
  "claude-code.strict": {
    id: "claude-code.strict",
    displayName: "Claude Code Strict",
    detail: "Project read/write only. Network denied. Child processes denied except git.",
    signatureStatus: "unsigned" as SignatureStatus,
    filesystem: {
      readableRoots: ["C:\\projects\\rampart"],
      writableRoots: ["C:\\projects\\rampart"],
      blockedRoots: ["C:\\Users", "C:\\Windows"],
    },
    network: {
      defaultAction: "deny" as DefaultAction,
      allowedHosts: [],
      blockedHosts: [],
    },
    process: {
      defaultAction: "deny" as DefaultAction,
      allowedCommands: ["git"],
      blockedCommands: ["powershell", "cmd"],
    },
  },
  "codex.standard": {
    id: "codex.standard",
    displayName: "Codex Standard",
    detail: "Project read/write, OpenAI API allowed. git, node, npm permitted.",
    signatureStatus: "unsigned" as SignatureStatus,
    filesystem: {
      readableRoots: ["C:\\projects\\rampart"],
      writableRoots: ["C:\\projects\\rampart"],
      blockedRoots: [],
    },
    network: {
      defaultAction: "deny" as DefaultAction,
      allowedHosts: ["api.openai.com"],
      blockedHosts: [],
    },
    process: {
      defaultAction: "deny" as DefaultAction,
      allowedCommands: ["git", "node", "npm"],
      blockedCommands: ["powershell"],
    },
  },
  "codex.strict": {
    id: "codex.strict",
    displayName: "Codex Strict",
    detail: "Project read/write only. Network denied. Only git permitted.",
    signatureStatus: "unsigned" as SignatureStatus,
    filesystem: {
      readableRoots: ["C:\\projects\\rampart"],
      writableRoots: ["C:\\projects\\rampart"],
      blockedRoots: ["C:\\Users", "C:\\Windows"],
    },
    network: {
      defaultAction: "deny" as DefaultAction,
      allowedHosts: [],
      blockedHosts: [],
    },
    process: {
      defaultAction: "deny" as DefaultAction,
      allowedCommands: ["git"],
      blockedCommands: ["powershell", "cmd"],
    },
  },
  "windows-safe": {
    id: "windows-safe",
    displayName: "Windows Safe",
    detail: "Project scoped read/write. Network denied by default.",
    signatureStatus: "unsigned" as SignatureStatus,
    filesystem: {
      readableRoots: ["C:\\projects\\rampart"],
      writableRoots: ["C:\\projects\\rampart\\apps"],
      blockedRoots: ["C:\\Users"],
    },
    network: {
      defaultAction: "deny" as DefaultAction,
      allowedHosts: [],
      blockedHosts: [],
    },
    process: {
      defaultAction: "deny" as DefaultAction,
      allowedCommands: ["git"],
      blockedCommands: ["powershell"],
    },
  },
  "windows-strict": {
    id: "windows-strict",
    displayName: "Windows Strict",
    detail: "Project read only outside src. Child process creation denied.",
    signatureStatus: "unsigned" as SignatureStatus,
    filesystem: {
      readableRoots: ["C:\\projects\\rampart"],
      writableRoots: ["C:\\projects\\rampart\\crates"],
      blockedRoots: ["C:\\Users", "C:\\Windows"],
    },
    network: {
      defaultAction: "deny" as DefaultAction,
      allowedHosts: [],
      blockedHosts: [],
    },
    process: {
      defaultAction: "deny" as DefaultAction,
      allowedCommands: ["git"],
      blockedCommands: ["powershell", "cmd"],
    },
  },
};

const profileStore = new Map<string, ProfileDetail>(Object.entries(mockProfileDetails));

function profilesForAgent(agentId: string | null): ProfileSummary[] {
  if (agentId && allProfilePresets[agentId]) {
    return allProfilePresets[agentId]!;
  }
  return genericProfiles;
}

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
      category: "session_lifecycle",
      message: "Session launched through mocked daemon contract.",
    },
    {
      id: `${sessionId}-audit-allow`,
      kind: "filesystem_allowed",
      category: "policy_enforcement",
      message: "Allowed read inside project scope: C:\\projects\\rampart\\README.md",
    },
    {
      id: `${sessionId}-audit-block`,
      kind: "filesystem_blocked",
      category: "policy_enforcement",
      message: "Blocked filesystem read: C:\\Users\\dev\\.ssh\\config",
    },
  ];
}

const mockExplanation: ViolationExplanation = {
  ruleDescription:
    "Access outside the allowed project roots is denied by the filesystem scope policy.",
  platformLimitation: {
    platform: "windows",
    engine: "rampart-windows-runtime-mock",
    detail:
      "Windows runtime enforcement support is not yet active. This violation was recorded by the mock adapter.",
  },
  remediationHint:
    "Adjust the profile's filesystem.readable_roots to include the target path if access is required.",
};

function buildViolations(sessionId: string): ViolationEvent[] {
  return [
    {
      id: `${sessionId}-violation-1`,
      operation: "read",
      target: "C:\\Users\\dev\\.ssh\\config",
      ruleId: "fs.scope.project-root-only",
      ruleLabel: "Filesystem access limited to selected project root.",
      message: "Policy denied read outside allowed project roots.",
      platformNote:
        "greywall is the reference adapter only. Windows runtime support remains unverified.",
      explanation: mockExplanation,
    },
  ];
}

let _selectedAgentId: string | null = mockAgents[0]?.id ?? null;

export const mockDaemonClient: DaemonApi = {
  async loadLaunchContext(): Promise<LaunchContext> {
    await pause(80);
    const profiles = profilesForAgent(_selectedAgentId);
    return {
      projects: mockProjects.map((project) => ({
        ...project,
        source: "mock",
      })),
      agents: mockAgents,
      profiles,
      selected: {
        projectPath: mockProjects[0]?.path ?? null,
        agentId: _selectedAgentId,
        profileId: profiles[0]?.id ?? null,
      },
      capabilities: capabilitySnapshot,
    };
  },
  async saveSelectedLaunchConfig(selected: SelectedLaunchConfig) {
    await pause(20);
    _selectedAgentId = selected.agentId;
  },
  async preflightCheck(_projectDir: string, _agentId: string, _profileId: string): Promise<PreflightReport> {
    await pause(40);
    return {
      ready: true,
      diagnostics: [
        { severity: "pass", label: "Project directory", detail: "Directory exists." },
        { severity: "warning", label: "Agent binary", detail: "codex not found on PATH." },
        { severity: "warning", label: "Filesystem enforcement", detail: "Unsupported on windows with greywall." },
      ],
    };
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
  async loadProfile(profileId: string): Promise<ProfileDetail> {
    await pause(40);
    const profile = profileStore.get(profileId);
    if (!profile) {
      throw new Error(`Profile not found: ${profileId}`);
    }
    return { ...profile };
  },
  async saveProfile(profile: ProfileDetail): Promise<void> {
    await pause(60);
    profileStore.set(profile.id, { ...profile });
  },
  async signProfile(profileId: string, _signingKeyB64: string): Promise<void> {
    await pause(60);
    const profile = profileStore.get(profileId);
    if (!profile) throw new Error(`Profile not found: ${profileId}`);
    profileStore.set(profileId, { ...profile, signatureStatus: "valid" as SignatureStatus });
  },
  async listSessionHistory(): Promise<SessionHistoryEntry[]> {
    await pause(40);
    return Array.from(sessions.entries()).map(([id, session]) => ({
      session: {
        ...session,
        startedAtMs: Date.now() - 60_000,
        endedAtMs: session.status === "stopped" ? Date.now() : null,
      },
      capabilitySnapshot: capabilitySnapshot,
      events: buildAudit(id),
      violations: buildViolations(id),
    }));
  },
};
