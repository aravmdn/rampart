import { useEffect, useState } from "react";
import {
  CapabilityPanel,
  EventList,
  HistoryDetailPanel,
  HistoryList,
  PickerSection,
  ProfileEditorPanel,
  SessionStatusPanel,
  StatusBadge,
  ViolationList,
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
  TASK3_API_NAMES,
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

type AppProps = {
  daemonClient?: DaemonApi;
};

function App({ daemonClient = tauriDaemonClient }: AppProps) {
  const [view, setView] = useState<AppView>("launcher");
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
  const [history, setHistory] = useState<SessionHistoryEntry[]>([]);
  const [selectedHistoryId, setSelectedHistoryId] = useState<string | null>(null);
  const [editorProfile, setEditorProfile] = useState<PolicyEditorView | null>(null);
  const [editorSuggestion, setEditorSuggestion] = useState<PolicySuggestion | undefined>(undefined);
  const [editorReturnView, setEditorReturnView] = useState<"launcher" | "session">("launcher");
  const [syncEndpoint, setSyncEndpoint] = useState("");
  const [syncToken, setSyncToken] = useState("");
  const [syncStripPaths, setSyncStripPaths] = useState(false);
  const [syncStatus, setSyncStatus] = useState<SyncStatus | null>(null);

  useEffect(() => {
    void (async () => {
      const launchContext = await daemonClient.loadLaunchContext();
      const recentHistory = await daemonClient.listSessionHistory();
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
    })();
  }, [daemonClient]);

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

  // Run preflight whenever selections change
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
      ? `${agent.detail} Terminal-first: interaction happens in the agent\u2019s own terminal.`
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

    setSession({
      id: null,
      status: "launching",
      projectPath: selectedProject.path,
      profileName: selectedProfile.displayName,
      agentName: selectedAgent.label,
    });
    setView("session");

    const launched = await daemonClient.launchSession({
      projectPath: selectedProject.path,
      agentId: selectedAgent.id,
      profileId: selectedProfile.id,
    });
    const sessionEvents = await daemonClient.streamSessionEvents(launched.id ?? "");
    const recentHistory = await daemonClient.listSessionHistory();

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
  }

  async function handleSyncSave() {
    await daemonClient.configureSync({ endpointUrl: syncEndpoint, token: syncToken, stripPaths: syncStripPaths });
    const status = await daemonClient.getSyncStatus();
    setSyncStatus(status);
  }

  async function handleSyncNow() {
    const status = await daemonClient.syncAuditEvents();
    setSyncStatus(status);
  }

  // ── Profile editor view ────────────────────────────────────────────
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

  // ── History view ───────────────────────────────────────────────────
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
          <button className="secondary-button" type="button" onClick={() => { setView("launcher"); setSelectedHistoryId(null); }}>
            Back to launcher
          </button>
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

  // ── Launcher view ──────────────────────────────────────────────────
  if (view === "launcher") {
    return (
      <main className="shell">
        <section className="panel hero">
          <p className="eyebrow">Rampart desktop shell</p>
          <h1>Launch console</h1>
          <p className="muted">
            Pick a project, agent, and profile. Review capability warnings and preflight diagnostics before launch.
          </p>
          <p className="muted">Mock API names for Task 3 sync: {Object.values(TASK3_API_NAMES).join(", ")}</p>
        </section>

        <div className="columns">
          <div className="column">
            <PickerSection
              title="Project picker"
              subtitle="Choose project scope before launch."
              items={projectItems}
              selectedId={projectId}
              onSelect={setProjectId}
            />
            <PickerSection
              title="Agent picker"
              subtitle="Choose supported agent tool."
              items={agentItems}
              selectedId={agentId}
              onSelect={setAgentId}
            />
            <PickerSection
              title="Profile picker"
              subtitle="Choose default-deny profile preset."
              items={profileItems}
              selectedId={profileId}
              onSelect={setProfileId}
            />
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
                <ul className="plain-list">
                  {preflight.diagnostics.map((diagnostic, index) => (
                    <li key={index}>
                      <strong>
                        {diagnostic.severity === "pass" ? "\u2713" : diagnostic.severity === "fail" ? "\u2717" : "\u26A0"}{" "}
                        {diagnostic.label}
                        {diagnostic.fromOrgPolicy ? (
                          <span> <span className="org-policy-tag">[from org policy]</span></span>
                        ) : null}
                      </strong>
                      <div className="muted">{diagnostic.detail}</div>
                    </li>
                  ))}
                </ul>
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
            </section>

            <section className="panel">
              <h2>Recent History</h2>
              {history.length === 0 ? (
                <p className="muted">No persisted sessions yet.</p>
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
                  <button
                    className="secondary-button"
                    type="button"
                    onClick={() => setView("history")}
                  >
                    View all history
                  </button>
                </>
              )}
            </section>
          </div>
        </div>
      </main>
    );
  }

  // ── Session console view ───────────────────────────────────────────
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
            <h2>Violation View</h2>
            {violations.length === 0 ? (
              <p className="muted">No blocked actions yet.</p>
            ) : (
              <ul className="plain-list">
                {violations.map((violation) => {
                  const suggestion = violationSuggestion(violation);
                  return (
                    <li key={violation.id}>
                      <strong>{violation.title}</strong>
                      <div>{violation.detail}</div>
                      <div className="muted">
                        Rule: {violation.policyRuleLabel} ({violation.policyRuleId})
                      </div>
                      {violation.platformNote ? (
                        <div className="muted">Platform note: {violation.platformNote}</div>
                      ) : null}
                      {suggestion ? (
                        <button
                          className="secondary-button"
                          type="button"
                          onClick={() => { void openProfileEditor("session", suggestion); }}
                        >
                          Adjust policy
                        </button>
                      ) : null}
                    </li>
                  );
                })}
              </ul>
            )}
          </section>
          <EventList events={events} />
        </div>
      </div>
    </main>
  );
}

export default App;
