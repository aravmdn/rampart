import { useEffect, useState } from "react";
import {
  CapabilityPanel,
  EventList,
  PickerSection,
  SessionStatusPanel,
  ViolationList,
  explainViolation,
  type CapabilityView,
  type EventView,
  type OptionItem,
  type SessionStatusView,
  type ViolationView,
} from "@rampart/shared-ui";
import "../../../packages/shared-ui/src/styles.css";
import "./styles.css";
import { mockAgents, mockDaemonClient, mockProjects } from "./daemon/mockDaemonClient";
import { TASK3_API_NAMES, type EngineCapabilitySnapshot, type ProfileSummary } from "./daemon/contracts";

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

function App() {
  const [projectId, setProjectId] = useState<string | null>(mockProjects[0]?.id ?? null);
  const [agentId, setAgentId] = useState<string | null>(mockAgents[0]?.id ?? null);
  const [profiles, setProfiles] = useState<ProfileSummary[]>([]);
  const [profileId, setProfileId] = useState<string | null>(null);
  const [snapshot, setSnapshot] = useState<EngineCapabilitySnapshot | null>(null);
  const [session, setSession] = useState<SessionStatusView>({
    id: null,
    status: "idle",
    projectPath: null,
    profileName: null,
    agentName: null,
  });
  const [events, setEvents] = useState<EventView[]>([]);
  const [violations, setViolations] = useState<ViolationView[]>([]);

  useEffect(() => {
    void (async () => {
      const [capabilities, availableProfiles] = await Promise.all([
        mockDaemonClient.detectCapabilities(),
        mockDaemonClient.listProfiles(),
      ]);
      setSnapshot(capabilities);
      setProfiles(availableProfiles);
      setProfileId(availableProfiles[0]?.id ?? null);
    })();
  }, []);

  const projectItems: OptionItem[] = mockProjects.map((project) => ({
    id: project.id,
    title: project.label,
    description: "Project root for sandboxed session.",
    meta: project.path,
  }));

  const agentItems: OptionItem[] = mockAgents.map((agent) => ({
    id: agent.id,
    title: agent.label,
    description: agent.detail,
  }));

  const profileItems: OptionItem[] = profiles.map((profile) => ({
    id: profile.id,
    title: profile.displayName,
    description: profile.detail,
  }));

  async function handleLaunch() {
    const selectedProject = mockProjects.find((project) => project.id === projectId);
    const selectedAgent = mockAgents.find((agent) => agent.id === agentId);
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

    const launched = await mockDaemonClient.launchSession({
      projectPath: selectedProject.path,
      agentId: selectedAgent.id,
      profileId: selectedProfile.id,
    });
    const sessionEvents = await mockDaemonClient.streamSessionEvents(launched.id ?? "");

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
        policyRuleLabel: "Project scope guard",
      })),
    );
  }

  async function handleStop() {
    if (!session.id) {
      return;
    }
    const stopped = await mockDaemonClient.stopSession(session.id);
    setSession((current) => ({
      ...current,
      status: stopped.status,
    }));
  }

  return (
    <main className="shell">
      <section className="panel hero">
        <p className="eyebrow">Rampart desktop shell</p>
        <h1>Windows-first session loop with mocked daemon truth</h1>
        <p className="muted">
          UI shows launch choices, capability gaps, audit stream, and blocked action explanation.
          Command construction stays outside view layer and stays daemon-owned.
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
          <section className="panel action-panel">
            <button className="launch-button" type="button" onClick={handleLaunch}>
              Launch session
            </button>
            <button className="secondary-button" type="button" onClick={handleStop} disabled={!session.id}>
              Stop session
            </button>
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
          <SessionStatusPanel value={session} />
          <ViolationList violations={violations} />
          <EventList events={events} />
        </div>
      </div>
    </main>
  );
}

export default App;
