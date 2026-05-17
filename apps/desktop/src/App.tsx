import { useEffect, useState, type ReactElement } from "react";
import {
  CapabilityPanel,
  EditIcon,
  EventList,
  FolderIcon,
  HistoryDetailPanel,
  HistoryIcon,
  HistoryList,
  MoonIcon,
  PickerSection,
  ProfileEditorPanel,
  SessionStatusPanel,
  ShieldIcon,
  StatusBadge,
  SunIcon,
  TerminalIcon,
  explainViolation,
  type CapabilityView,
  type EventView,
  type HistorySessionView,
  type OptionItem,
  type PolicyEditorView,
  type PolicySuggestion,
  type SessionStatusView,
  type ViolationView,
} from "@rampart/shared-ui";
import "../../../packages/shared-ui/src/styles.css";
import "./styles.css";
import { tauriDaemonClient } from "./daemon/tauriDaemonClient";
import {
  type AgentTool,
  type DaemonApi,
  type EngineCapabilitySnapshot,
  type PreflightReport,
  type ProfileDetail,
  type ProfileSummary,
  type ProjectSummary,
  type SessionHistoryEntry,
  type SyncStatus,
} from "./daemon/contracts";

type AppView = "launcher" | "session" | "history" | "profile-editor";
type Theme = "light" | "dark";
const THEME_STORAGE_KEY = "rampart.theme";

const VIEW_LABELS: Record<AppView, string> = {
  launcher: "Launcher",
  session: "Active session",
  history: "History",
  "profile-editor": "Profile editor",
};

function toCapabilityView(snapshot: EngineCapabilitySnapshot): CapabilityView[] {
  const labels: Record<string, string> = {
    filesystem_scope: "Filesystem scope",
    network_egress: "Network egress",
    process_execution: "Process execution",
    violation_streaming: "Violation streaming",
    session_termination: "Session termination",
  };

  return snapshot.capabilities.map((capability) => ({
    key: capability.key,
    label: labels[capability.key] ?? capability.key,
    status: capability.status,
    detail: capability.detail,
  }));
}

function hasUnsupported(snapshot: EngineCapabilitySnapshot | null): boolean {
  if (!snapshot) return false;
  return snapshot.capabilities.some((c) => c.status === "unsupported");
}

function formatMs(ms: number): string {
  return new Date(ms).toLocaleString();
}

function formatTime(ms: number): string {
  return new Date(ms).toLocaleTimeString();
}

function formatDuration(startMs: number, endMs: number | null): string {
  if (endMs === null) return "ongoing";
  const secs = Math.round((endMs - startMs) / 1000);
  if (secs < 60) return `${secs}s`;
  return `${Math.floor(secs / 60)}m ${secs % 60}s`;
}

function toHistorySessionView(
  entry: SessionHistoryEntry,
  explainViolationFn: (v: { operation: string; target: string; ruleId?: string; ruleLabel?: string; platformNote?: string | null; message?: string }) => string,
): HistorySessionView {
  return {
    id: entry.session.id ?? "unknown",
    startedAt: formatMs(entry.session.startedAtMs),
    duration: formatDuration(entry.session.startedAtMs, entry.session.endedAtMs),
    agentId: entry.session.agentId ?? "unknown",
    profileId: entry.session.profileId ?? "unknown",
    projectPath: entry.session.projectPath ?? "",
    eventCount: entry.events.length,
    violationCount: entry.violations.length,
    events: entry.events.map((e) => ({ id: e.id, label: e.kind, message: e.message })),
    violations: entry.violations.map((v) => ({
      id: v.id,
      title: `${v.operation} blocked`,
      detail: explainViolationFn(v),
      policyRuleId: v.ruleId,
      policyRuleLabel: v.ruleLabel ?? "Project scope guard",
      platformNote: v.platformNote ?? undefined,
      operation: v.operation,
      target: v.target,
    })),
  };
}

function initialTheme(): Theme {
  if (typeof window === "undefined") return "light";
  try {
    const stored = window.localStorage.getItem(THEME_STORAGE_KEY);
    if (stored === "light" || stored === "dark") return stored;
  } catch {
    // localStorage may be blocked in some embeds; fall through to media query.
  }
  if (typeof window.matchMedia === "function") {
    return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
  }
  return "light";
}

type AppProps = {
  daemonClient?: DaemonApi;
};

function App({ daemonClient = tauriDaemonClient }: AppProps) {
  const [view, setView] = useState<AppView>("launcher");
  const [theme, setTheme] = useState<Theme>(() => initialTheme());
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [agents, setAgents] = useState<AgentTool[]>([]);
  const [projectId, setProjectId] = useState<string | null>(null);
  const [agentId, setAgentId] = useState<string | null>(null);
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);
  const [profileId, setProfileId] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<EngineCapabilitySnapshot | null>(null);
  const [preflight, setPreflight] = useState<PreflightReport | null>(null);
  const [session, setSession] = useState<SessionStatusView>({
    id: null,
    status: "idle",
    projectPath: null,
    profileName: null,
    agentName: null,
  });
  const [events, setEvents] = useState<EventView[]>([]);
  const [violations, setViolations] = useState<ViolationView[]>([]);
  const [lastEventAtMs, setLastEventAtMs] = useState<number | null>(null);
  const [history, setHistory] = useState<SessionHistoryEntry[]>([]);
  const [selectedHistoryId, setSelectedHistoryId] = useState<string | null>(null);
  const [editorProfile, setEditorProfile] = useState<PolicyEditorView | null>(null);
  const [editorSuggestion, setEditorSuggestion] = useState<PolicySuggestion | undefined>(undefined);
  const [editorReturnView, setEditorReturnView] = useState<"launcher" | "session">("launcher");
  const [syncEndpoint, setSyncEndpoint] = useState("");
  const [syncToken, setSyncToken] = useState("");
  const [syncStripPaths, setSyncStripPaths] = useState(false);
  const [syncStatus, setSyncStatus] = useState<SyncStatus | null>(null);
  const [orgPolicyUrl, setOrgPolicyUrl] = useState("");
  const [orgPolicy, setOrgPolicy] = useState<import("./daemon/contracts").OrgPolicy | null>(null);
  const [launchError, setLaunchError] = useState<string | null>(null);
  const [addProjectPath, setAddProjectPath] = useState("");
  const [addProjectError, setAddProjectError] = useState<string | null>(null);
  const [orgPolicyError, setOrgPolicyError] = useState<string | null>(null);
  const [syncError, setSyncError] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [profilesLoading, setProfilesLoading] = useState(false);

  useEffect(() => {
    if (typeof document === "undefined") return;
    document.documentElement.dataset.theme = theme;
    try {
      window.localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // ignore storage failures
    }
  }, [theme]);

  useEffect(() => {
    if (typeof window === "undefined" || typeof window.matchMedia !== "function") return;
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (event: MediaQueryListEvent) => {
      try {
        if (window.localStorage.getItem(THEME_STORAGE_KEY)) return;
      } catch {
        // ignore
      }
      setTheme(event.matches ? "dark" : "light");
    };
    if (typeof mq.addEventListener === "function") {
      mq.addEventListener("change", handler);
      return () => mq.removeEventListener("change", handler);
    }
    mq.addListener(handler);
    return () => mq.removeListener(handler);
  }, []);

  useEffect(() => {
    void (async () => {
      const launchContext = await daemonClient.loadLaunchContext();
      const recentHistory = await daemonClient.listSessionHistory();
      const currentOrg = await daemonClient.currentOrgPolicy();
      const initialSync = await daemonClient.getSyncStatus().catch(() => null);
      setOrgPolicy(currentOrg);
      setSyncStatus(initialSync);
      setProjects(launchContext.projects);
      setAgents(launchContext.agents);
      setProfiles(launchContext.profiles);
      setSnapshot(launchContext.capabilities);
      setHistory(recentHistory);
      setProjectId(
        launchContext.projects.find((project) => project.path === launchContext.selected.projectPath)?.id ??
          launchContext.projects[0]?.id ??
          null,
      );
      setAgentId(launchContext.selected.agentId ?? launchContext.agents[0]?.id ?? null);
      setProfileId(launchContext.selected.profileId ?? launchContext.profiles[0]?.id ?? null);
      setLoaded(true);
    })();
  }, [daemonClient]);

  useEffect(() => {
    if (!loaded || !agentId) return;
    setProfilesLoading(true);
    void daemonClient.loadLaunchContext().then((ctx) => {
      setProfiles(ctx.profiles);
      setProfileId((prev) =>
        prev && ctx.profiles.some((p) => p.id === prev) ? prev : ctx.profiles[0]?.id ?? null,
      );
      setProfilesLoading(false);
    });
  }, [agentId, daemonClient, loaded]);

  useEffect(() => {
    const selectedProject = projects.find((project) => project.id === projectId);
    if (!selectedProject || !agentId || !profileId) {
      return;
    }
    void daemonClient.saveSelectedLaunchConfig({
      projectPath: selectedProject.path,
      agentId,
      profileId,
    });
  }, [agentId, daemonClient, profileId, projectId, projects]);

  useEffect(() => {
    if (!session.id) return;
    if (session.status === "stopped" || session.status === "failed" || session.status === "idle") return;
    const interval = setInterval(() => {
      void daemonClient.streamSessionEvents(session.id!).then((sessionEvents) => {
        setEvents(
          sessionEvents.audit.map((event) => ({
            id: event.id,
            label: event.kind,
            message: event.message,
          })),
        );
        if (sessionEvents.audit.length > 0) {
          setLastEventAtMs(Date.now());
        }
        setViolations(
          sessionEvents.violations.map((violation) => ({
            id: violation.id,
            title: `${violation.operation} blocked`,
            detail: explainViolation(violation),
            policyRuleId: violation.ruleId,
            policyRuleLabel: violation.ruleLabel ?? "Project scope guard",
            platformNote: violation.platformNote ?? undefined,
            operation: violation.operation,
            target: violation.target,
          })),
        );
      });
    }, 3000);
    return () => clearInterval(interval);
  }, [daemonClient, session.id, session.status]);

  useEffect(() => {
    const selectedProject = projects.find((project) => project.id === projectId);
    if (!selectedProject || !agentId || !profileId) {
      setPreflight(null);
      return;
    }
    void daemonClient.preflightCheck(selectedProject.path, agentId, profileId).then(setPreflight);
  }, [agentId, daemonClient, profileId, projectId, projects]);

  const projectItems: OptionItem[] = projects.map((project) => ({
    id: project.id,
    title: project.label,
    description: "Project root for sandboxed session.",
    meta: project.path,
  }));

  const agentItems: OptionItem[] = agents.map((agent) => ({
    id: agent.id,
    title: agent.label,
    description: agent.terminalFirst
      ? `${agent.detail} Terminal-first: interaction happens in the agent’s own terminal.`
      : agent.detail,
  }));

  const profileItems: OptionItem[] = profiles.map((profile) => ({
    id: profile.id,
    title: profile.displayName,
    description:
      profile.signatureStatus === "valid"
        ? `[signed] ${profile.detail}`
        : profile.signatureStatus === "invalid"
          ? `[signature invalid] ${profile.detail}`
          : profile.detail,
  }));

  const selectedAgent = agents.find((agent) => agent.id === agentId);

  const historySessions: HistorySessionView[] = history.map((entry) =>
    toHistorySessionView(entry, explainViolation),
  );

  function profileDetailToEditorView(p: ProfileDetail): PolicyEditorView {
    return {
      id: p.id,
      displayName: p.displayName,
      detail: p.detail,
      filesystem: p.filesystem,
      network: p.network,
      process: p.process,
    };
  }

  async function openProfileEditor(returnView: "launcher" | "session", suggestion?: PolicySuggestion) {
    if (!profileId) return;
    const detail = await daemonClient.loadProfile(profileId);
    let editor = profileDetailToEditorView(detail);
    if (suggestion) {
      const section = suggestion.section as keyof PolicyEditorView;
      if (section === "filesystem" || section === "network" || section === "process") {
        const field = suggestion.field as keyof typeof editor[typeof section];
        const list = editor[section][field] as string[];
        if (!list.includes(suggestion.value)) {
          (editor[section] as Record<string, unknown>)[field] = [...list, suggestion.value];
        }
      }
    }
    setEditorProfile(editor);
    setEditorSuggestion(suggestion);
    setEditorReturnView(returnView);
    setView("profile-editor");
  }

  async function handleSaveProfile(updated: PolicyEditorView) {
    const detail: ProfileDetail = { ...updated, signatureStatus: "unsigned" };
    await daemonClient.saveProfile(detail);
    setProfiles((prev) =>
      prev.map((p) =>
        p.id === updated.id
          ? { id: updated.id, displayName: updated.displayName, detail: updated.detail, signatureStatus: "unsigned" as const }
          : p,
      ),
    );
    setEditorProfile(null);
    setEditorSuggestion(undefined);
    setView(editorReturnView);
  }

  function handleCancelEditor() {
    setEditorProfile(null);
    setEditorSuggestion(undefined);
    setView(editorReturnView);
  }

  function violationSuggestion(v: ViolationView): PolicySuggestion | undefined {
    if (!v.operation || !v.target) return undefined;
    if (v.operation === "read") {
      return {
        section: "filesystem",
        field: "readableRoots",
        value: v.target,
        reason: `The agent tried to read "${v.target}" but it was outside the allowed readable paths. Add it to allow access.`,
      };
    }
    if (v.operation === "write") {
      return {
        section: "filesystem",
        field: "writableRoots",
        value: v.target,
        reason: `The agent tried to write to "${v.target}" but it was outside the allowed writable paths. Add it to allow writes.`,
      };
    }
    if (v.operation === "network") {
      return {
        section: "network",
        field: "allowedHosts",
        value: v.target,
        reason: `The agent tried to reach "${v.target}" but network access was blocked. Add it to allow this host.`,
      };
    }
    if (v.operation === "execute") {
      return {
        section: "process",
        field: "allowedCommands",
        value: v.target,
        reason: `The agent tried to run "${v.target}" but the command was blocked. Add it to allow execution.`,
      };
    }
    return undefined;
  }

  async function handleLaunch() {
    const selectedProject = projects.find((project) => project.id === projectId);
    const selectedProfile = profiles.find((profile) => profile.id === profileId);

    if (!selectedProject || !selectedAgent || !selectedProfile) {
      return;
    }

    setLaunchError(null);
    setLastEventAtMs(null);
    setSession({
      id: null,
      status: "launching",
      projectPath: selectedProject.path,
      profileName: selectedProfile.displayName,
      agentName: selectedAgent.label,
    });
    setView("session");

    try {
      const launched = await daemonClient.launchSession({
        projectPath: selectedProject.path,
        agentId: selectedAgent.id,
        profileId: selectedProfile.id,
      });
      const sessionEvents = await daemonClient.streamSessionEvents(launched.id ?? "");
      const recentHistory = await daemonClient.listSessionHistory();

      if (launched.status === "failed") {
        const lastMsg = sessionEvents.audit.length > 0
          ? sessionEvents.audit[sessionEvents.audit.length - 1].message
          : "Session launch failed (no detail). Check that the agent is installed on PATH and that you ran Rampart as Administrator.";
        setLaunchError(lastMsg);
      }

      setSession({
        id: launched.id,
        status: launched.status,
        projectPath: launched.projectPath,
        profileName: selectedProfile.displayName,
        agentName: selectedAgent.label,
      });
      setEvents(
        sessionEvents.audit.map((event) => ({
          id: event.id,
          label: event.kind,
          message: event.message,
        })),
      );
      if (sessionEvents.audit.length > 0) {
        setLastEventAtMs(Date.now());
      }
      setViolations(
        sessionEvents.violations.map((violation) => ({
          id: violation.id,
          title: `${violation.operation} blocked`,
          detail: explainViolation(violation),
          policyRuleId: violation.ruleId,
          policyRuleLabel: violation.ruleLabel ?? "Project scope guard",
          platformNote: violation.platformNote ?? undefined,
          operation: violation.operation,
          target: violation.target,
        })),
      );
      setHistory(recentHistory);
    } catch (err) {
      setSession({
        id: null,
        status: "failed",
        projectPath: selectedProject.path,
        profileName: selectedProfile.displayName,
        agentName: selectedAgent.label,
      });
      setLaunchError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleStop() {
    if (!session.id) {
      return;
    }
    const stopped = await daemonClient.stopSession(session.id);
    const recentHistory = await daemonClient.listSessionHistory();
    setSession((current) => ({
      ...current,
      status: stopped.status,
    }));
    setHistory(recentHistory);
  }

  function handleBackToLauncher() {
    setView("launcher");
    setSession({ id: null, status: "idle", projectPath: null, profileName: null, agentName: null });
    setEvents([]);
    setViolations([]);
    setLastEventAtMs(null);
    setLaunchError(null);
  }

  async function handleAddProject() {
    const path = addProjectPath.trim();
    setAddProjectError(null);
    if (!path) {
      setAddProjectError("Enter a project directory path before adding.");
      return;
    }
    try {
      await daemonClient.addProject(path);
      setAddProjectPath("");
      const launchContext = await daemonClient.loadLaunchContext();
      setProjects(launchContext.projects);
      const justAdded = launchContext.projects.find((p) => p.path === path);
      if (justAdded) setProjectId(justAdded.id);
    } catch (err) {
      setAddProjectError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleRemoveProject(path: string) {
    const wasSelected = projects.find((p) => p.id === projectId)?.path === path;
    await daemonClient.removeProject(path);
    const launchContext = await daemonClient.loadLaunchContext();
    setProjects(launchContext.projects);
    if (wasSelected) {
      setProjectId(launchContext.projects[0]?.id ?? null);
    }
  }

  async function handleOrgPolicySave() {
    setOrgPolicyError(null);
    try {
      await daemonClient.configureOrgPolicyUrl(orgPolicyUrl.trim() || null);
    } catch (err) {
      setOrgPolicyError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleOrgPolicyFetch() {
    setOrgPolicyError(null);
    try {
      const policy = await daemonClient.fetchOrgPolicy();
      setOrgPolicy(policy);
    } catch (err) {
      setOrgPolicyError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleSyncSave() {
    setSyncError(null);
    try {
      await daemonClient.configureSync({ endpointUrl: syncEndpoint, token: syncToken, stripPaths: syncStripPaths });
      const status = await daemonClient.getSyncStatus();
      setSyncStatus(status);
    } catch (err) {
      setSyncError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleSyncNow() {
    setSyncError(null);
    try {
      const status = await daemonClient.syncAuditEvents();
      setSyncStatus(status);
    } catch (err) {
      setSyncError(err instanceof Error ? err.message : String(err));
    }
  }

  function toggleTheme() {
    setTheme((current) => (current === "dark" ? "light" : "dark"));
  }

  const sidebarItems: { id: AppView; label: string; icon: ReactElement }[] = [
    { id: "launcher", label: "Launcher", icon: <FolderIcon size={18} /> },
    { id: "session", label: "Active session", icon: <TerminalIcon size={18} /> },
    { id: "history", label: "History", icon: <HistoryIcon size={18} /> },
    { id: "profile-editor", label: "Profile editor", icon: <EditIcon size={18} /> },
  ];

  function navigateTo(target: AppView) {
    if (target === "profile-editor") {
      void openProfileEditor("launcher");
      return;
    }
    if (target === "session") {
      setView("session");
      return;
    }
    if (target === "history") {
      setView("history");
      return;
    }
    setView("launcher");
  }

  return (
    <div className="app-frame">
      <header className="app-header" role="banner">
        <div className="brand">
          <span className="brand-mark" aria-hidden>
            <ShieldIcon size={22} />
          </span>
          <div className="brand-text">
            <span className="brand-name">Rampart</span>
            <span className="brand-tagline">Least-privilege sandbox for AI coding agents</span>
          </div>
        </div>
        <div className="header-actions">
          <span className="current-view-pill" aria-label="Current view">
            {VIEW_LABELS[view]}
          </span>
          <button
            type="button"
            className="icon-button"
            aria-label={theme === "dark" ? "Switch to light theme" : "Switch to dark theme"}
            aria-pressed={theme === "dark"}
            onClick={toggleTheme}
            title={theme === "dark" ? "Switch to light theme" : "Switch to dark theme"}
          >
            {theme === "dark" ? <SunIcon size={18} /> : <MoonIcon size={18} />}
          </button>
        </div>
      </header>

      <nav className="tab-bar" aria-label="Views (compact)">
        {sidebarItems.map((item) => (
          <button
            key={item.id}
            type="button"
            className={`sidebar-item ${view === item.id ? "sidebar-item-active" : ""}`}
            onClick={() => navigateTo(item.id)}
          >
            <span className="sidebar-item-icon">{item.icon}</span>
            <span>{item.label}</span>
          </button>
        ))}
      </nav>

      <div className="app-body">
        <aside className="sidebar" aria-label="Main navigation">
          <span className="sidebar-section-label">Workspace</span>
          {sidebarItems.map((item) => (
            <button
              key={item.id}
              type="button"
              className={`sidebar-item ${view === item.id ? "sidebar-item-active" : ""}`}
              aria-current={view === item.id ? "page" : undefined}
              onClick={() => navigateTo(item.id)}
            >
              <span className="sidebar-item-icon">{item.icon}</span>
              <span>{item.label}</span>
            </button>
          ))}
          <span className="sidebar-spacer" />
        </aside>

        <ViewContent
          view={view}
          loaded={loaded}
          editorProfile={editorProfile}
          editorSuggestion={editorSuggestion}
          historySessions={historySessions}
          selectedHistoryId={selectedHistoryId}
          setSelectedHistoryId={setSelectedHistoryId}
          setView={setView}
          handleSaveProfile={handleSaveProfile}
          handleCancelEditor={handleCancelEditor}
          projects={projects}
          projectId={projectId}
          setProjectId={setProjectId}
          projectItems={projectItems}
          agentItems={agentItems}
          agentId={agentId}
          setAgentId={setAgentId}
          profileItems={profileItems}
          profileId={profileId}
          setProfileId={setProfileId}
          profilesLoading={profilesLoading}
          snapshot={snapshot}
          preflight={preflight}
          selectedAgent={selectedAgent}
          addProjectPath={addProjectPath}
          setAddProjectPath={setAddProjectPath}
          handleAddProject={handleAddProject}
          handleRemoveProject={handleRemoveProject}
          addProjectError={addProjectError}
          openProfileEditor={openProfileEditor}
          handleLaunch={handleLaunch}
          syncEndpoint={syncEndpoint}
          setSyncEndpoint={setSyncEndpoint}
          syncToken={syncToken}
          setSyncToken={setSyncToken}
          syncStripPaths={syncStripPaths}
          setSyncStripPaths={setSyncStripPaths}
          syncStatus={syncStatus}
          handleSyncSave={handleSyncSave}
          handleSyncNow={handleSyncNow}
          syncError={syncError}
          orgPolicyUrl={orgPolicyUrl}
          setOrgPolicyUrl={setOrgPolicyUrl}
          orgPolicy={orgPolicy}
          handleOrgPolicySave={handleOrgPolicySave}
          handleOrgPolicyFetch={handleOrgPolicyFetch}
          orgPolicyError={orgPolicyError}
          history={history}
          session={session}
          events={events}
          violations={violations}
          lastEventAtMs={lastEventAtMs}
          launchError={launchError}
          handleStop={handleStop}
          handleBackToLauncher={handleBackToLauncher}
          violationSuggestion={violationSuggestion}
        />
      </div>
    </div>
  );
}

type ViewContentProps = {
  view: AppView;
  loaded: boolean;
  editorProfile: PolicyEditorView | null;
  editorSuggestion: PolicySuggestion | undefined;
  historySessions: HistorySessionView[];
  selectedHistoryId: string | null;
  setSelectedHistoryId: (id: string | null) => void;
  setView: (v: AppView) => void;
  handleSaveProfile: (updated: PolicyEditorView) => Promise<void>;
  handleCancelEditor: () => void;
  projects: ProjectSummary[];
  projectId: string | null;
  setProjectId: (id: string | null) => void;
  projectItems: OptionItem[];
  agentItems: OptionItem[];
  agentId: string | null;
  setAgentId: (id: string | null) => void;
  profileItems: OptionItem[];
  profileId: string | null;
  setProfileId: (id: string | null) => void;
  profilesLoading: boolean;
  snapshot: EngineCapabilitySnapshot | null;
  preflight: PreflightReport | null;
  selectedAgent: AgentTool | undefined;
  addProjectPath: string;
  setAddProjectPath: (s: string) => void;
  handleAddProject: () => Promise<void>;
  handleRemoveProject: (path: string) => Promise<void>;
  addProjectError: string | null;
  openProfileEditor: (returnView: "launcher" | "session", suggestion?: PolicySuggestion) => Promise<void>;
  handleLaunch: () => Promise<void>;
  syncEndpoint: string;
  setSyncEndpoint: (s: string) => void;
  syncToken: string;
  setSyncToken: (s: string) => void;
  syncStripPaths: boolean;
  setSyncStripPaths: (b: boolean) => void;
  syncStatus: SyncStatus | null;
  handleSyncSave: () => Promise<void>;
  handleSyncNow: () => Promise<void>;
  syncError: string | null;
  orgPolicyUrl: string;
  setOrgPolicyUrl: (s: string) => void;
  orgPolicy: import("./daemon/contracts").OrgPolicy | null;
  handleOrgPolicySave: () => Promise<void>;
  handleOrgPolicyFetch: () => Promise<void>;
  orgPolicyError: string | null;
  history: SessionHistoryEntry[];
  session: SessionStatusView;
  events: EventView[];
  violations: ViolationView[];
  lastEventAtMs: number | null;
  launchError: string | null;
  handleStop: () => Promise<void>;
  handleBackToLauncher: () => void;
  violationSuggestion: (v: ViolationView) => PolicySuggestion | undefined;
};

function ViewContent(props: ViewContentProps) {
  const {
    view,
    loaded,
    editorProfile,
    editorSuggestion,
    historySessions,
    selectedHistoryId,
    setSelectedHistoryId,
    setView,
    handleSaveProfile,
    handleCancelEditor,
    projects,
    projectId,
    setProjectId,
    projectItems,
    agentItems,
    agentId,
    setAgentId,
    profileItems,
    profileId,
    setProfileId,
    profilesLoading,
    snapshot,
    preflight,
    selectedAgent,
    addProjectPath,
    setAddProjectPath,
    handleAddProject,
    handleRemoveProject,
    addProjectError,
    openProfileEditor,
    handleLaunch,
    syncEndpoint,
    setSyncEndpoint,
    syncToken,
    setSyncToken,
    syncStripPaths,
    setSyncStripPaths,
    syncStatus,
    handleSyncSave,
    handleSyncNow,
    syncError,
    orgPolicyUrl,
    setOrgPolicyUrl,
    orgPolicy,
    handleOrgPolicySave,
    handleOrgPolicyFetch,
    orgPolicyError,
    history,
    session,
    events,
    violations,
    lastEventAtMs,
    launchError,
    handleStop,
    handleBackToLauncher,
    violationSuggestion,
  } = props;

  if (view === "profile-editor" && editorProfile) {
    return (
      <main className="shell">
        <section className="panel hero">
          <p className="eyebrow">Rampart profile editor</p>
          <h1>Edit profile</h1>
          <p className="muted">
            Adjust policy rules in product language. Changes apply to future sessions using this profile.
          </p>
        </section>
        <div className="columns">
          <div className="column">
            <ProfileEditorPanel
              profile={editorProfile}
              suggestion={editorSuggestion}
              onSave={(updated) => { void handleSaveProfile(updated); }}
              onCancel={handleCancelEditor}
            />
          </div>
        </div>
      </main>
    );
  }

  if (view === "history") {
    const selectedHistorySession = selectedHistoryId
      ? historySessions.find((s) => s.id === selectedHistoryId) ?? null
      : null;

    return (
      <main className="shell">
        <section className="panel hero">
          <p className="eyebrow">Rampart session history</p>
          <h1>Session history</h1>
          <p className="muted">
            Inspect past sessions, blocked actions, and audit events.
          </p>
          <div className="action-row">
            <button className="secondary-button" type="button" onClick={() => { setView("launcher"); setSelectedHistoryId(null); }}>
              Back to launcher
            </button>
          </div>
        </section>

        <div className="columns">
          <div className="column">
            <HistoryList
              sessions={historySessions}
              selectedId={selectedHistoryId}
              onSelect={setSelectedHistoryId}
            />
          </div>

          <div className="column">
            {selectedHistorySession ? (
              <HistoryDetailPanel
                session={selectedHistorySession}
                onBack={() => setSelectedHistoryId(null)}
              />
            ) : (
              <section className="panel">
                <p className="muted">Select a session to inspect its events and violations.</p>
              </section>
            )}
          </div>
        </div>
      </main>
    );
  }

  if (view === "launcher") {
    if (!loaded) {
      return (
        <main className="shell">
          <section className="panel hero">
            <p className="eyebrow">Rampart desktop shell</p>
            <h1>Launch console</h1>
            <p className="muted">Loading projects, agents, and profiles…</p>
          </section>
        </main>
      );
    }
    return (
      <main className="shell">
        <section className="panel hero">
          <p className="eyebrow">Rampart desktop shell</p>
          <h1>Launch console</h1>
          <p className="muted">
            Pick a project, agent, and profile. Review capability warnings and preflight diagnostics before launch.
          </p>
        </section>

        <div className="columns">
          <div className="column">
            {projectItems.length === 0 ? (
              <section className="panel">
                <h2>Project picker</h2>
                <p className="muted">No projects added yet — pick one with the button below.</p>
              </section>
            ) : (
              <PickerSection
                title="Project picker"
                subtitle="Choose project scope before launch."
                items={projectItems}
                selectedId={projectId}
                onSelect={setProjectId}
              />
            )}
            <section className="panel">
              <h2>Add project</h2>
              <label className="field-label">
                Project path
                <input
                  className="field-input"
                  type="text"
                  placeholder="C:\path\to\project"
                  value={addProjectPath}
                  onChange={(e) => setAddProjectPath(e.target.value)}
                  onKeyDown={(e) => { if (e.key === "Enter") { void handleAddProject(); } }}
                />
              </label>
              <div className="action-row">
                <button className="secondary-button" type="button" onClick={() => { void handleAddProject(); }}>
                  Add
                </button>
              </div>
              {addProjectError ? <p className="muted error-text">{addProjectError}</p> : null}
              {projects.filter((p) => p.source === "user-added").length > 0 ? (
                <ul className="plain-list">
                  {projects.filter((p) => p.source === "user-added").map((p) => (
                    <li key={p.id}>
                      <div className="history-row">
                        <div>
                          <span>{p.label}</span>
                          <div className="muted">{p.path}</div>
                        </div>
                        <button
                          className="secondary-button"
                          type="button"
                          onClick={() => { void handleRemoveProject(p.path); }}
                        >
                          Remove
                        </button>
                      </div>
                    </li>
                  ))}
                </ul>
              ) : null}
            </section>
            <PickerSection
              title="Agent picker"
              subtitle="Choose supported agent tool."
              items={agentItems}
              selectedId={agentId}
              onSelect={setAgentId}
            />
            {profileItems.length === 0 ? (
              <section className="panel">
                <h2>Profile picker</h2>
                <p className="muted">
                  {profilesLoading ? "Loading profiles…" : "No profiles available for the selected agent."}
                </p>
              </section>
            ) : (
              <PickerSection
                title="Profile picker"
                subtitle="Choose default-deny profile preset."
                items={profileItems}
                selectedId={profileId}
                onSelect={setProfileId}
              />
            )}
            {profileId ? (
              <section className="panel">
                <button
                  className="secondary-button"
                  type="button"
                  onClick={() => { void openProfileEditor("launcher"); }}
                >
                  Edit selected profile
                </button>
              </section>
            ) : null}

            {selectedAgent?.terminalFirst ? (
              <section className="panel">
                <h2>Terminal Handoff</h2>
                <p className="muted">
                  {selectedAgent.label} is a terminal-first agent. Rampart will launch the session and apply enforcement,
                  but interaction happens in the agent&rsquo;s own terminal. Rampart stays open for live session state,
                  violation visibility, and stop control.
                </p>
              </section>
            ) : null}

            <section className="panel action-panel">
              <button
                className="launch-button"
                type="button"
                onClick={handleLaunch}
                disabled={!preflight?.ready}
              >
                Launch session
              </button>
              {preflight && !preflight.ready ? (
                <p className="muted">Preflight checks must pass before launch.</p>
              ) : null}
            </section>
          </div>

          <div className="column">
            {snapshot ? (
              <CapabilityPanel
                platformLabel={snapshot.platform}
                engineName={snapshot.engineName}
                capabilities={toCapabilityView(snapshot)}
              />
            ) : null}
            {hasUnsupported(snapshot) ? (
              <section className="panel">
                <h2>Capability Warnings</h2>
                <p className="muted">
                  Some enforcement capabilities are unsupported on this platform and engine combination.
                  Policy rules in unsupported domains will not be enforced during the session.
                  Review the capability snapshot above before launching.
                </p>
              </section>
            ) : null}

            {preflight ? (
              <section className="panel">
                <div className="panel-header">
                  <h2>Preflight Diagnostics</h2>
                  {preflight.diagnostics.some((d) => d.fromOrgPolicy) ? (
                    <StatusBadge tone="warn">Org policy floor active</StatusBadge>
                  ) : null}
                </div>
                {preflight.diagnostics.length === 0 ? (
                  <p className="muted">No diagnostics for this configuration.</p>
                ) : (
                  <ul className="plain-list">
                    {preflight.diagnostics.map((diagnostic, index) => (
                      <li key={index}>
                        <strong>
                          {diagnostic.severity === "pass" ? "✓" : diagnostic.severity === "fail" ? "✗" : "⚠"}{" "}
                          {diagnostic.label}
                          {diagnostic.fromOrgPolicy ? (
                            <span> <span className="org-policy-tag">[from org policy]</span></span>
                          ) : null}
                        </strong>
                        <div className="muted">{diagnostic.detail}</div>
                      </li>
                    ))}
                  </ul>
                )}
              </section>
            ) : null}

            <section className="panel">
              <h2>Audit Sync</h2>
              <p className="muted">
                Push local audit events to a team endpoint. Configure below and click Save, then Sync Now.
              </p>
              <label className="field-label">
                Endpoint URL
                <input
                  className="field-input"
                  type="text"
                  placeholder="https://your-endpoint/api/v1/audit/events"
                  value={syncEndpoint}
                  onChange={(e) => setSyncEndpoint(e.target.value)}
                />
              </label>
              <label className="field-label">
                Bearer token
                <input
                  className="field-input"
                  type="password"
                  placeholder="team token"
                  value={syncToken}
                  onChange={(e) => setSyncToken(e.target.value)}
                />
              </label>
              <label className="field-label checkbox-label">
                <input
                  type="checkbox"
                  checked={syncStripPaths}
                  onChange={(e) => setSyncStripPaths(e.target.checked)}
                />
                {" "}Strip file paths before sending
              </label>
              <div className="action-row">
                <button className="secondary-button" type="button" onClick={() => { void handleSyncSave(); }}>
                  Save
                </button>
                <button className="secondary-button" type="button" onClick={() => { void handleSyncNow(); }} disabled={!syncStatus?.configured}>
                  Sync now
                </button>
              </div>
              {syncStatus ? (
                <div className="muted">
                  {syncStatus.configured ? `Configured · Queue: ${syncStatus.queueDepth}` : "Not configured"}
                  {syncStatus.lastSyncAtMs ? ` · Last sync: ${formatMs(syncStatus.lastSyncAtMs)}` : ""}
                  {syncStatus.lastError ? ` · Error: ${syncStatus.lastError}` : ""}
                </div>
              ) : null}
              {syncError ? <p className="muted error-text">{syncError}</p> : null}
            </section>

            <section className="panel">
              <h2>Org Policy</h2>
              <p className="muted">
                Point Rampart at a remote org policy URL. The policy floor is fetched and merged with the local profile at preflight time.
              </p>
              <label className="field-label">
                Policy URL
                <input
                  className="field-input"
                  type="text"
                  placeholder="https://your-org/api/v1/policy.json"
                  value={orgPolicyUrl}
                  onChange={(e) => setOrgPolicyUrl(e.target.value)}
                />
              </label>
              <div className="action-row">
                <button className="secondary-button" type="button" onClick={() => { void handleOrgPolicySave(); }}>
                  Save URL
                </button>
                <button className="secondary-button" type="button" onClick={() => { void handleOrgPolicyFetch(); }}>
                  Fetch policy
                </button>
              </div>
              {orgPolicy ? (
                <div className="muted">
                  Active: {orgPolicy.name}
                  {orgPolicy.description ? ` — ${orgPolicy.description}` : ""}
                </div>
              ) : (
                <div className="muted">No org policy active.</div>
              )}
              {orgPolicyError ? <p className="muted error-text">{orgPolicyError}</p> : null}
            </section>

            <section className="panel">
              <h2>Recent History</h2>
              {history.length === 0 ? (
                <p className="muted">No prior sessions — launch a sandboxed session to populate history.</p>
              ) : (
                <>
                  <ul className="plain-list">
                    {history.slice(0, 3).map((entry) => (
                      <li key={entry.session.id}>
                        <strong>{entry.session.id}</strong>
                        <div className="muted">{entry.session.projectPath}</div>
                        {entry.events[0] ? <div>{entry.events[0].message}</div> : null}
                        {entry.violations[0] ? (
                          <div className="muted">
                            Latest block: {entry.violations[0].target} ({entry.violations[0].ruleId})
                          </div>
                        ) : null}
                      </li>
                    ))}
                  </ul>
                  <div className="action-row">
                    <button
                      className="secondary-button"
                      type="button"
                      onClick={() => setView("history")}
                    >
                      View all history
                    </button>
                  </div>
                </>
              )}
            </section>
          </div>
        </div>
      </main>
    );
  }

  return (
    <main className="shell">
      <section className="panel hero">
        <p className="eyebrow">Rampart session console</p>
        <h1>Active session</h1>
        <p className="muted">
          Live session state, audit events, and blocked action explanations.
        </p>
      </section>

      <div className="columns">
        <div className="column">
          <SessionStatusPanel value={session} />
          {launchError ? (
            <section className="panel">
              <h2>Launch error</h2>
              <p className="muted">{launchError}</p>
            </section>
          ) : null}
          <section className="panel action-panel">
            <button className="secondary-button" type="button" onClick={handleStop} disabled={!session.id}>
              Stop session
            </button>
            <button
              className="secondary-button"
              type="button"
              onClick={handleBackToLauncher}
              disabled={session.status === "active" || session.status === "launching"}
            >
              Back to launcher
            </button>
          </section>
        </div>

        <div className="column">
          <section className="panel">
            <div className="panel-header">
              <h2>Violation View</h2>
              <span className="rampart-session-pill" aria-label="Violation count">
                <span className="rampart-session-pill-dot" />
                {violations.length} block{violations.length === 1 ? "" : "s"}
              </span>
            </div>
            {violations.length === 0 ? (
              <p className="muted">No blocked actions yet.</p>
            ) : (
              <ul className="rampart-event-list">
                {violations.map((violation) => {
                  const suggestion = violationSuggestion(violation);
                  return (
                    <li key={violation.id} className="rampart-event-item rampart-event-item--violation">
                      <strong>{violation.title}</strong>
                      <div>{violation.detail}</div>
                      <div className="muted">
                        Rule: {violation.policyRuleLabel} ({violation.policyRuleId})
                      </div>
                      {violation.platformNote ? (
                        <div className="muted">Platform note: {violation.platformNote}</div>
                      ) : null}
                      {suggestion ? (
                        <div className="action-row">
                          <button
                            className="secondary-button"
                            type="button"
                            onClick={() => { void openProfileEditor("session", suggestion); }}
                          >
                            Adjust policy
                          </button>
                        </div>
                      ) : null}
                    </li>
                  );
                })}
              </ul>
            )}
          </section>
          <EventList
            events={events}
            showLivePill
            lastEventAt={lastEventAtMs ? formatTime(lastEventAtMs) : null}
          />
        </div>
      </div>
    </main>
  );
}

export default App;
