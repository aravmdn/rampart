import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
}));

describe("tauri daemon client", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("loads launch context through tauri invoke and maps snake_case to camelCase", async () => {
    // Mock data matches actual Rust serialization (snake_case, no rename_all on these structs)
    invoke.mockResolvedValue({
      projects: [
        { id: "proj-1", label: "My Project", path: "C:\\projects\\test", source: "detected" },
      ],
      agents: [
        { id: "claude-code", label: "Claude Code", detail: "Anthropic agent.", terminal_first: true },
      ],
      profiles: [
        { id: "claude-code.standard", display_name: "Claude Code Standard", detail: "Standard.", signature_status: "unsigned" },
      ],
      selected: {
        project_path: "C:\\projects\\test",
        agent_id: "claude-code",
        profile_id: "claude-code.standard",
      },
      capabilities: {
        engine_name: "rampart-windows",
        platform: "windows",
      },
      capability_items: [
        { key: "network_egress", status: "supported", detail: "WFP outbound block active." },
      ],
    });

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    const ctx = await tauriDaemonClient.loadLaunchContext();

    expect(invoke).toHaveBeenCalledWith("load_launch_context");

    // Verify the mapping layer converts snake_case to camelCase correctly
    expect(ctx.agents[0]!.terminalFirst).toBe(true);
    expect(ctx.profiles[0]!.displayName).toBe("Claude Code Standard");
    expect(ctx.profiles[0]!.signatureStatus).toBe("unsigned");
    expect(ctx.selected.projectPath).toBe("C:\\projects\\test");
    expect(ctx.selected.agentId).toBe("claude-code");
    expect(ctx.selected.profileId).toBe("claude-code.standard");
    expect(ctx.capabilities.engineName).toBe("rampart-windows");
    expect(ctx.capabilities.capabilities[0]!.key).toBe("network_egress");
  });

  it("persists selected launch config through tauri invoke", async () => {
    invoke.mockResolvedValue(null);

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    await tauriDaemonClient.saveSelectedLaunchConfig({
      projectPath: "C:\\projects\\rampart",
      agentId: "codex",
      profileId: "windows-safe",
    });

    expect(invoke).toHaveBeenCalledWith("save_selected_launch_config", {
      selected: {
        projectPath: "C:\\projects\\rampart",
        agentId: "codex",
        profileId: "windows-safe",
      },
    });
  });

  it("maps streamSessionEvents violation fields from Rust snake_case to camelCase", async () => {
    invoke.mockResolvedValue({
      audit: [
        {
          kind: "filesystem-blocked",
          category: "policy-enforcement",
          message: "Blocked read: C:\\Users\\dev\\.ssh\\config",
          session_id: "session-1",
          sequence: 1,
          occurred_at_ms: 1700000001000,
        },
      ],
      violations: [
        {
          action: "read",
          target: "C:\\Users\\dev\\.ssh\\config",
          rule_id: "fs.scope.project-root-only",
          rule_label: "Filesystem scope",
          reason: "Policy denied read outside allowed project roots.",
          platform_note: "WFP block active.",
          explanation: {
            rule_description: "Access outside the allowed project roots is denied.",
            platform_limitation: {
              platform: "windows",
              engine: "rampart-windows",
              detail: "WFP enforces at kernel level.",
            },
            remediation_hint: "Add the target path to filesystem.readable_roots.",
          },
        },
      ],
    });

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    const result = await tauriDaemonClient.streamSessionEvents("session-1");

    // Audit mapping: kind and category are normalized through the kind/category maps
    expect(result.audit).toHaveLength(1);
    expect(result.audit[0]!.kind).toBe("filesystem_blocked");
    expect(result.audit[0]!.category).toBe("policy_enforcement");
    expect(result.audit[0]!.message).toBe("Blocked read: C:\\Users\\dev\\.ssh\\config");

    // Violation mapping: action→operation, reason→message, rule_id→ruleId, etc.
    expect(result.violations).toHaveLength(1);
    const v = result.violations[0]!;
    expect(v.operation).toBe("read");
    expect(v.target).toBe("C:\\Users\\dev\\.ssh\\config");
    expect(v.ruleId).toBe("fs.scope.project-root-only");
    expect(v.ruleLabel).toBe("Filesystem scope");
    expect(v.message).toBe("Policy denied read outside allowed project roots.");
    expect(v.platformNote).toBe("WFP block active.");
    expect(v.explanation).not.toBeNull();
    expect(v.explanation!.ruleDescription).toBe("Access outside the allowed project roots is denied.");
    expect(v.explanation!.platformLimitation).not.toBeNull();
    expect(v.explanation!.platformLimitation!.platform).toBe("windows");
    expect(v.explanation!.remediationHint).toBe("Add the target path to filesystem.readable_roots.");
  });

  it("maps listSessionHistory entries from Rust snake_case to camelCase", async () => {
    invoke.mockResolvedValue([
      {
        session: {
          id: "hist-1",
          status: "finished",
          profile_id: "claude-code.standard",
          project_path: "C:\\projects\\rampart",
          started_at_ms: 1700000000000,
          ended_at_ms: 1700000030000,
        },
        capability_snapshot: {
          engine_name: "rampart-windows",
          platform: "windows",
          capability_items: [
            { key: "network_egress", status: "partial", detail: "WFP active." },
          ],
        },
        events: [
          {
            kind: "session-ended",
            category: "session-lifecycle",
            message: "Session ended.",
            session_id: "hist-1",
            sequence: 5,
            occurred_at_ms: 1700000030000,
          },
        ],
        violations: [
          {
            action: "network",
            target: "evil.example.com:443",
            rule_id: "net.blocked-host",
            rule_label: "Network egress",
            reason: "Host in blocked list.",
            platform_note: null,
            explanation: null,
          },
        ],
      },
    ]);

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    const history = await tauriDaemonClient.listSessionHistory();

    expect(history).toHaveLength(1);
    const entry = history[0]!;

    // Session fields
    expect(entry.session.id).toBe("hist-1");
    expect(entry.session.status).toBe("stopped");  // "finished" maps to "stopped"
    expect(entry.session.profileId).toBe("claude-code.standard");
    expect(entry.session.projectPath).toBe("C:\\projects\\rampart");
    expect(entry.session.startedAtMs).toBe(1700000000000);
    expect(entry.session.endedAtMs).toBe(1700000030000);

    // Capability snapshot
    expect(entry.capabilitySnapshot).not.toBeNull();
    expect(entry.capabilitySnapshot!.engineName).toBe("rampart-windows");
    expect(entry.capabilitySnapshot!.capabilities).toHaveLength(1);
    expect(entry.capabilitySnapshot!.capabilities[0]!.key).toBe("network_egress");

    // Event mapping
    expect(entry.events).toHaveLength(1);
    expect(entry.events[0]!.kind).toBe("session_stopped");
    expect(entry.events[0]!.category).toBe("session_lifecycle");

    // Violation mapping
    expect(entry.violations).toHaveLength(1);
    const v = entry.violations[0]!;
    expect(v.operation).toBe("network");
    expect(v.ruleId).toBe("net.blocked-host");
    expect(v.ruleLabel).toBe("Network egress");
    expect(v.message).toBe("Host in blocked list.");
    expect(v.platformNote).toBeNull();
    expect(v.explanation).toBeNull();
  });

  it("maps preflightCheck fromOrgPolicy field from Rust snake_case", async () => {
    invoke.mockResolvedValue({
      ready: true,
      diagnostics: [
        { severity: "pass", label: "Project directory", detail: "Exists.", from_org_policy: false },
        { severity: "warning", label: "Network blocked", detail: "Org policy requires network deny.", from_org_policy: true },
      ],
    });

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    const report = await tauriDaemonClient.preflightCheck("C:\\projects\\test", "claude-code", "claude-code.standard");

    expect(report.ready).toBe(true);
    expect(report.diagnostics).toHaveLength(2);
    expect(report.diagnostics[0]!.fromOrgPolicy).toBe(false);
    expect(report.diagnostics[1]!.fromOrgPolicy).toBe(true);
    expect(report.diagnostics[1]!.severity).toBe("warning");
    expect(report.diagnostics[1]!.label).toBe("Network blocked");
  });
});
