import { useState } from "react";
import type {
  CapabilityView,
  EventView,
  HistorySessionView,
  OptionItem,
  PolicyEditorView,
  PolicySuggestion,
  SessionStatusView,
  ViolationView,
} from "./types";
import { StatusBadge, type StatusBadgeTone } from "../components/StatusBadge";
import { AlertIcon, CircleDotIcon, TerminalIcon } from "./icons";

function eventAccentClass(label: string): string {
  const k = label.toLowerCase();
  if (k.includes("violation") || k.includes("block")) return "rampart-event-item--violation";
  if (k.includes("network") || k.includes("connect") || k.includes("dns") || k.includes("http")) {
    return "rampart-event-item--network";
  }
  if (k.includes("file") || k.includes("read") || k.includes("write") || k.includes("path") || k.includes("fs")) {
    return "rampart-event-item--filesystem";
  }
  if (k.includes("process") || k.includes("exec") || k.includes("spawn") || k.includes("command")) {
    return "rampart-event-item--process";
  }
  if (k.includes("session") || k.includes("lifecycle") || k.includes("launch") || k.includes("stop") || k.includes("exit")) {
    return "rampart-event-item--lifecycle";
  }
  return "rampart-event-item--lifecycle";
}

function capabilityTone(status: CapabilityView["status"]): StatusBadgeTone {
  if (status === "supported") return "success";
  if (status === "unsupported") return "error";
  return "warn";
}

type PickerSectionProps = {
  title: string;
  subtitle: string;
  items: OptionItem[];
  selectedId: string | null;
  onSelect: (id: string) => void;
};

export function PickerSection({
  title,
  subtitle,
  items,
  selectedId,
  onSelect,
}: PickerSectionProps) {
  return (
    <section className="panel">
      <h2>{title}</h2>
      <p className="muted">{subtitle}</p>
      <div className="list-grid">
        {items.map((item) => (
          <button
            key={item.id}
            className={`choice ${selectedId === item.id ? "choice-selected" : ""}`}
            type="button"
            onClick={() => onSelect(item.id)}
          >
            <span className="choice-name">{item.title}</span>
            <span className="choice-desc">{item.description}</span>
            {item.meta ? <span className="choice-meta">{item.meta}</span> : null}
          </button>
        ))}
      </div>
    </section>
  );
}

type CapabilityPanelProps = {
  platformLabel: string;
  engineName: string;
  capabilities: CapabilityView[];
};

export function CapabilityPanel({
  platformLabel,
  engineName,
  capabilities,
}: CapabilityPanelProps) {
  const hasGaps = capabilities.some((c) => c.status !== "supported");
  return (
    <section className="panel">
      <div className="panel-header">
        <div>
          <h2>Capability Snapshot</h2>
          <p className="muted">
            {platformLabel} | {engineName}
          </p>
        </div>
        {hasGaps ? (
          <StatusBadge tone="warn">Gaps present</StatusBadge>
        ) : (
          <StatusBadge tone="success">All supported</StatusBadge>
        )}
      </div>
      <ul className="plain-list">
        {capabilities.map((capability) => (
          <li key={capability.key}>
            <strong>{capability.label}</strong>{" "}
            <StatusBadge tone={capabilityTone(capability.status)}>
              {capability.status}
            </StatusBadge>
            <div className="muted">{capability.detail}</div>
          </li>
        ))}
      </ul>
    </section>
  );
}

type SessionStatusPanelProps = {
  value: SessionStatusView;
};

export function SessionStatusPanel({ value }: SessionStatusPanelProps) {
  const tone: StatusBadgeTone =
    value.status === "active"
      ? "success"
      : value.status === "failed"
        ? "error"
        : value.status === "launching"
          ? "info"
          : "warn";

  return (
    <section className="panel">
      <div className="panel-header">
        <h2>Session Status</h2>
        <StatusBadge tone={tone}>{value.status}</StatusBadge>
      </div>
      <p className="muted">Session: {value.id ?? "none"}</p>
      <p className="muted">Project: {value.projectPath ?? "none selected"}</p>
      <p className="muted">Agent: {value.agentName ?? "none selected"}</p>
      <p className="muted">Profile: {value.profileName ?? "none selected"}</p>
    </section>
  );
}

type EventListProps = {
  events: EventView[];
  showLivePill?: boolean;
  lastEventAt?: string | null;
};

export function EventList({ events, showLivePill = false, lastEventAt = null }: EventListProps) {
  return (
    <section className="panel">
      <div className="panel-header">
        <h2>Audit Stream</h2>
        {showLivePill ? (
          <span className="rampart-session-pill" aria-label="Live event counter">
            <span className="rampart-session-pill-dot" />
            {events.length} event{events.length === 1 ? "" : "s"}
            {lastEventAt ? <span className="muted"> · {lastEventAt}</span> : null}
          </span>
        ) : null}
      </div>
      {events.length === 0 ? (
        <div className="rampart-empty-state">
          <TerminalIcon size={28} className="rampart-empty-state-icon" />
          <p>No events yet — agent has just started.</p>
        </div>
      ) : (
        <ul className="rampart-event-list">
          {events.map((event) => (
            <li key={event.id} className={`rampart-event-item ${eventAccentClass(event.label)}`}>
              <span className="rampart-event-kind">{event.label}</span>
              <div className="rampart-event-message">{event.message}</div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

type ViolationListProps = {
  violations: ViolationView[];
};

export function ViolationList({ violations }: ViolationListProps) {
  return (
    <section className="panel">
      <h2>Violation View</h2>
      {violations.length === 0 ? (
        <div className="rampart-empty-state">
          <CircleDotIcon size={28} className="rampart-empty-state-icon" />
          <p>No blocked actions yet.</p>
        </div>
      ) : (
        <ul className="rampart-event-list">
          {violations.map((violation) => (
            <li key={violation.id} className="rampart-event-item rampart-event-item--violation">
              <strong>{violation.title}</strong>
              <div>{violation.detail}</div>
              <div className="muted">
                Rule: {violation.policyRuleLabel} ({violation.policyRuleId})
              </div>
              {violation.platformNote ? (
                <div className="muted">Platform note: {violation.platformNote}</div>
              ) : null}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

type HistoryListProps = {
  sessions: HistorySessionView[];
  selectedId: string | null;
  onSelect: (id: string) => void;
};

export function HistoryList({ sessions, selectedId, onSelect }: HistoryListProps) {
  return (
    <section className="panel">
      <h2>Session History</h2>
      {sessions.length === 0 ? (
        <div className="rampart-empty-state">
          <AlertIcon size={28} className="rampart-empty-state-icon" />
          <p>No prior sessions — launch a sandboxed session from the launcher to populate history.</p>
        </div>
      ) : (
        <ul className="plain-list">
          {sessions.map((session) => (
            <li key={session.id} className={selectedId === session.id ? "history-selected" : ""}>
              <div className="history-row">
                <div>
                  <strong>{session.startedAt}</strong>
                  <span className="muted"> — {session.duration}</span>
                </div>
                <button
                  className="secondary-button"
                  type="button"
                  onClick={() => onSelect(session.id)}
                >
                  View
                </button>
              </div>
              <div className="muted">
                {session.agentId} · {session.profileId}
              </div>
              <div className="muted">{session.projectPath}</div>
              <div className="muted">
                {session.eventCount} event{session.eventCount !== 1 ? "s" : ""},{" "}
                {session.violationCount} violation{session.violationCount !== 1 ? "s" : ""}
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

type ProfileEditorPanelProps = {
  profile: PolicyEditorView;
  suggestion?: PolicySuggestion;
  onSave: (updated: PolicyEditorView) => void;
  onCancel: () => void;
};

function listToText(items: string[]): string {
  return items.join("\n");
}

function textToList(text: string): string[] {
  return text
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

export function ProfileEditorPanel({
  profile,
  suggestion,
  onSave,
  onCancel,
}: ProfileEditorPanelProps) {
  const [readableRoots, setReadableRoots] = useState(listToText(profile.filesystem.readableRoots));
  const [writableRoots, setWritableRoots] = useState(listToText(profile.filesystem.writableRoots));
  const [blockedRoots, setBlockedRoots] = useState(listToText(profile.filesystem.blockedRoots));
  const [networkDefault, setNetworkDefault] = useState(profile.network.defaultAction);
  const [allowedHosts, setAllowedHosts] = useState(listToText(profile.network.allowedHosts));
  const [blockedHosts, setBlockedHosts] = useState(listToText(profile.network.blockedHosts));
  const [processDefault, setProcessDefault] = useState(profile.process.defaultAction);
  const [allowedCommands, setAllowedCommands] = useState(listToText(profile.process.allowedCommands));
  const [blockedCommands, setBlockedCommands] = useState(listToText(profile.process.blockedCommands));

  function handleSave() {
    onSave({
      ...profile,
      filesystem: {
        readableRoots: textToList(readableRoots),
        writableRoots: textToList(writableRoots),
        blockedRoots: textToList(blockedRoots),
      },
      network: {
        defaultAction: networkDefault,
        allowedHosts: textToList(allowedHosts),
        blockedHosts: textToList(blockedHosts),
      },
      process: {
        defaultAction: processDefault,
        allowedCommands: textToList(allowedCommands),
        blockedCommands: textToList(blockedCommands),
      },
    });
  }

  return (
    <>
      {suggestion ? (
        <section className="panel">
          <h2>Suggested rule change</h2>
          <p className="muted">{suggestion.reason}</p>
          <p>
            <strong>Add to {suggestion.field}:</strong>{" "}
            <code>{suggestion.value}</code>
          </p>
          <p className="muted">Review and adjust the policy below, then save.</p>
        </section>
      ) : null}

      <section className="panel">
        <div className="panel-header">
          <div>
            <h2>{profile.displayName}</h2>
            <p className="muted">{profile.detail}</p>
          </div>
        </div>

        <h3>Filesystem</h3>
        <label className="editor-label">
          Paths the agent can read (one per line)
          <textarea
            className="editor-textarea"
            value={readableRoots}
            onChange={(e) => setReadableRoots(e.target.value)}
            rows={3}
          />
        </label>
        <label className="editor-label">
          Paths the agent can write (one per line)
          <textarea
            className="editor-textarea"
            value={writableRoots}
            onChange={(e) => setWritableRoots(e.target.value)}
            rows={3}
          />
        </label>
        <label className="editor-label">
          Always blocked paths (one per line)
          <textarea
            className="editor-textarea"
            value={blockedRoots}
            onChange={(e) => setBlockedRoots(e.target.value)}
            rows={2}
          />
        </label>

        <h3>Network</h3>
        <label className="editor-label">
          Default network action
          <div className="toggle-row">
            <button
              className={`toggle-button ${networkDefault === "deny" ? "toggle-active" : ""}`}
              type="button"
              onClick={() => setNetworkDefault("deny")}
            >
              Block all by default
            </button>
            <button
              className={`toggle-button ${networkDefault === "allow" ? "toggle-active" : ""}`}
              type="button"
              onClick={() => setNetworkDefault("allow")}
            >
              Allow all by default
            </button>
          </div>
        </label>
        <label className="editor-label">
          Allowed hosts (one per line)
          <textarea
            className="editor-textarea"
            value={allowedHosts}
            onChange={(e) => setAllowedHosts(e.target.value)}
            rows={3}
          />
        </label>
        <label className="editor-label">
          Always blocked hosts (one per line)
          <textarea
            className="editor-textarea"
            value={blockedHosts}
            onChange={(e) => setBlockedHosts(e.target.value)}
            rows={2}
          />
        </label>

        <h3>Process execution</h3>
        <label className="editor-label">
          Default for running commands
          <div className="toggle-row">
            <button
              className={`toggle-button ${processDefault === "deny" ? "toggle-active" : ""}`}
              type="button"
              onClick={() => setProcessDefault("deny")}
            >
              Block all by default
            </button>
            <button
              className={`toggle-button ${processDefault === "allow" ? "toggle-active" : ""}`}
              type="button"
              onClick={() => setProcessDefault("allow")}
            >
              Allow all by default
            </button>
          </div>
        </label>
        <label className="editor-label">
          Allowed commands (one per line)
          <textarea
            className="editor-textarea"
            value={allowedCommands}
            onChange={(e) => setAllowedCommands(e.target.value)}
            rows={3}
          />
        </label>
        <label className="editor-label">
          Always blocked commands (one per line)
          <textarea
            className="editor-textarea"
            value={blockedCommands}
            onChange={(e) => setBlockedCommands(e.target.value)}
            rows={2}
          />
        </label>

        <div className="action-row">
          <button className="launch-button" type="button" onClick={handleSave}>
            Save profile
          </button>
          <button className="secondary-button" type="button" onClick={onCancel}>
            Cancel
          </button>
        </div>
      </section>
    </>
  );
}

type HistoryDetailPanelProps = {
  session: HistorySessionView;
  onBack: () => void;
};

export function HistoryDetailPanel({ session, onBack }: HistoryDetailPanelProps) {
  return (
    <>
      <section className="panel">
        <div className="panel-header">
          <div>
            <h2>Session Detail</h2>
            <p className="muted">{session.id}</p>
          </div>
          <button className="secondary-button" type="button" onClick={onBack}>
            Back to history
          </button>
        </div>
        <p className="muted">Started: {session.startedAt}</p>
        <p className="muted">Duration: {session.duration}</p>
        <p className="muted">Agent: {session.agentId}</p>
        <p className="muted">Profile: {session.profileId}</p>
        <p className="muted">Project: {session.projectPath}</p>
      </section>
      <ViolationList violations={session.violations} />
      <EventList events={session.events} />
    </>
  );
}
