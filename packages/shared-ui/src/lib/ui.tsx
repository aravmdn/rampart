import type {
  CapabilityView,
  EventView,
  OptionItem,
  SessionStatusView,
  ViolationView,
} from "./types";
import { StatusBadge } from "../components/StatusBadge";

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
  return (
    <section className="panel">
      <div className="panel-header">
        <div>
          <h2>Capability Snapshot</h2>
          <p className="muted">
            {platformLabel} | {engineName}
          </p>
        </div>
        <StatusBadge tone="warn">Show gaps before launch</StatusBadge>
      </div>
      <ul className="plain-list">
        {capabilities.map((capability) => (
          <li key={capability.key}>
            <strong>{capability.label}</strong>{" "}
            <StatusBadge tone={capability.status === "supported" ? "info" : "warn"}>
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
  const tone = value.status === "active" ? "info" : value.status === "failed" ? "error" : "warn";

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
};

export function EventList({ events }: EventListProps) {
  return (
    <section className="panel">
      <h2>Audit Stream</h2>
      {events.length === 0 ? (
        <p className="muted">No audit events yet.</p>
      ) : (
        <ul className="plain-list">
          {events.map((event) => (
            <li key={event.id}>
              <strong>{event.label}</strong>
              <div>{event.message}</div>
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
        <p className="muted">No blocked actions yet.</p>
      ) : (
        <ul className="plain-list">
          {violations.map((violation) => (
            <li key={violation.id}>
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
