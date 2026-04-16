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
