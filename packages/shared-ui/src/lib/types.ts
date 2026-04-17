export type OptionItem = {
  id: string;
  title: string;
  description: string;
  meta?: string;
};

export type CapabilityView = {
  key: string;
  label: string;
  status: "supported" | "unsupported" | "partial";
  detail: string;
};

export type SessionStatus = "idle" | "launching" | "active" | "stopped" | "failed";

export type SessionStatusView = {
  id: string | null;
  status: SessionStatus;
  projectPath: string | null;
  profileName: string | null;
  agentName: string | null;
};

export type EventView = {
  id: string;
  label: string;
  message: string;
};

export type ViolationView = {
  id: string;
  title: string;
  detail: string;
  policyRuleId: string;
  policyRuleLabel: string;
  platformNote?: string;
  operation?: string;
  target?: string;
};

export type DefaultAction = "allow" | "deny";

export type PolicyEditorView = {
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
};

export type PolicySuggestionField =
  | "readableRoots"
  | "writableRoots"
  | "blockedRoots"
  | "allowedHosts"
  | "blockedHosts"
  | "allowedCommands"
  | "blockedCommands";

export type PolicySuggestion = {
  section: "filesystem" | "network" | "process";
  field: PolicySuggestionField;
  value: string;
  reason: string;
};

export type HistorySessionView = {
  id: string;
  startedAt: string;
  duration: string;
  agentId: string;
  profileId: string;
  projectPath: string;
  eventCount: number;
  violationCount: number;
  events: EventView[];
  violations: ViolationView[];
};
