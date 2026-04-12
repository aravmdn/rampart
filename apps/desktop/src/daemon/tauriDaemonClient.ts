import { invoke } from "@tauri-apps/api/core";
import type {
  DaemonApi,
  LaunchContext,
  LaunchSessionRequest,
  SelectedLaunchConfig,
  SessionHistoryEntry,
  SessionState,
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
  }[];
  profiles: {
    id: string;
    display_name: string;
    detail: string;
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
};

type RawHistoryEntry = {
  session: {
    id: string;
    status: string;
    profile_id: string;
    project_path: string;
    started_at_ms: number;
    ended_at_ms: number | null;
  };
  events: {
    kind: string;
    message: string;
    session_id: string;
    sequence: number;
    occurred_at_ms: number;
  }[];
  violations: {
    id?: string;
    action: "read" | "write" | "execute" | "network";
    target: string;
    rule_id: string;
    reason: string;
  }[];
};

function mapLaunchContext(raw: RawLaunchContext): LaunchContext {
  return {
    projects: raw.projects,
    agents: raw.agents,
    profiles: raw.profiles.map((profile) => ({
      id: profile.id,
      displayName: profile.display_name,
      detail: profile.detail,
    })),
    selected: {
      projectPath: raw.selected.project_path,
      agentId: raw.selected.agent_id,
      profileId: raw.selected.profile_id,
    },
    capabilities: {
      engineName: raw.capabilities.engine_name,
      platform: raw.capabilities.platform as "windows" | "macos" | "linux",
      capabilities: [],
    },
  };
}

function mapSessionState(raw: {
  id: string;
  status: string;
  profile_id: string;
  project_path: string;
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
    agentId: null,
    projectPath: raw.project_path,
  };
}

export const tauriDaemonClient: DaemonApi = {
  async loadLaunchContext() {
    const response = await invoke<RawLaunchContext>("load_launch_context");
    return mapLaunchContext(response);
  },
  async saveSelectedLaunchConfig(selected: SelectedLaunchConfig) {
    await invoke("save_selected_launch_config", { selected });
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
    return invoke<{ audit: any[]; violations: any[] }>("stream_session_events", { sessionId });
  },
  async listSessionHistory() {
    const response = await invoke<RawHistoryEntry[]>("list_session_history");
    return response.map((entry) => ({
      session: {
        id: entry.session.id,
        status: mapSessionState(entry.session).status,
        profileId: entry.session.profile_id,
        agentId: null,
        projectPath: entry.session.project_path,
        startedAtMs: entry.session.started_at_ms,
        endedAtMs: entry.session.ended_at_ms,
      },
      events: entry.events.map((event, index) => ({
        id: `${entry.session.id}-event-${index}`,
        kind:
          event.kind === "violation-recorded"
            ? "block_observed"
            : event.kind === "session-ended"
              ? "session_stopped"
              : "allow_observed",
        message: event.message,
      })),
      violations: entry.violations.map((violation, index) => ({
        id: `${entry.session.id}-violation-${index}`,
        operation: violation.action,
        target: violation.target,
        ruleId: violation.rule_id,
        message: violation.reason,
      })),
    }));
  },
};
