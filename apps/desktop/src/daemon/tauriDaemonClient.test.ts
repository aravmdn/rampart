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

  it("maps launchSession and stopSession SessionState from Rust snake_case", async () => {
    invoke.mockResolvedValueOnce({
      id: "sess-1",
      status: "running",
      agent_tool: "ClaudeCode",
      profile_id: "claude-code.standard",
      project_path: "C:\\projects\\test",
    });
    invoke.mockResolvedValueOnce({
      id: "sess-1",
      status: "finished",
      agent_tool: "Codex",
      profile_id: "claude-code.standard",
      project_path: "C:\\projects\\test",
    });

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    const launched = await tauriDaemonClient.launchSession({
      projectPath: "C:\\projects\\test",
      agentId: "claude-code",
      profileId: "claude-code.standard",
      isolationMode: "windows-native",
    });
    const stopped = await tauriDaemonClient.stopSession("sess-1");

    expect(invoke).toHaveBeenCalledWith("launch_session", expect.anything());
    expect(launched.id).toBe("sess-1");
    expect(launched.status).toBe("active");
    expect(launched.agentId).toBe("claude-code");  // "ClaudeCode" → "claude-code"
    expect(launched.profileId).toBe("claude-code.standard");
    expect(launched.projectPath).toBe("C:\\projects\\test");

    expect(stopped.status).toBe("stopped");  // "finished" maps to "stopped"
    expect(stopped.agentId).toBe("codex");   // "Codex" → "codex"
  });

  it("maps agent_tool enum variants to string IDs in launchSession", async () => {
    const variants: Array<[unknown, string]> = [
      ["ClaudeCode", "claude-code"],
      ["Codex", "codex"],
      ["Cursor", "cursor"],
      ["Copilot", "copilot"],
      ["Aider", "aider"],
      ["Goose", "goose"],
      ["OpenCode", "opencode"],
      ["GeminiCli", "gemini"],
      [{ Custom: { id: "my-agent" } }, "my-agent"],
    ];

    const { tauriDaemonClient } = await import("./tauriDaemonClient");

    for (const [agentTool, expectedId] of variants) {
      invoke.mockResolvedValueOnce({
        id: "s",
        status: "running",
        agent_tool: agentTool,
        profile_id: "p",
        project_path: "C:\\projects\\test",
      });
      const session = await tauriDaemonClient.launchSession({
        projectPath: "C:\\projects\\test",
        agentId: "x",
        profileId: "p",
      });
      expect(session.agentId).toBe(expectedId);
    }
  });

  it("maps getSyncStatus snake_case fields to camelCase", async () => {
    invoke.mockResolvedValue({
      configured: true,
      queue_depth: 42,
      last_sync_at_ms: 1700000099000,
      last_error: null,
    });

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    const status = await tauriDaemonClient.getSyncStatus();

    expect(invoke).toHaveBeenCalledWith("get_sync_status");
    expect(status.configured).toBe(true);
    expect(status.queueDepth).toBe(42);
    expect(status.lastSyncAtMs).toBe(1700000099000);
    expect(status.lastError).toBeNull();
  });

  it("maps fetchOrgPolicy nested snake_case fields to camelCase", async () => {
    invoke.mockResolvedValue({
      id: "org-policy-1",
      name: "Acme Corp Policy",
      description: "Standard policy.",
      scope: {
        agent_types: ["claude-code", "codex"],
        project_path_glob: "C:\\projects\\*",
      },
      policy: {
        filesystem: {
          readable_roots: ["C:\\projects"],
          writable_roots: ["C:\\projects\\out"],
          blocked_roots: ["C:\\Users\\dev\\.ssh"],
        },
        network: {
          default_action: "deny",
          allowed_hosts: ["api.github.com"],
          blocked_hosts: ["evil.example.com"],
        },
        process: {
          default_action: "allow",
          allowed_commands: ["git", "npm"],
          blocked_commands: ["rm"],
        },
      },
      signature: null,
    });

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    const policy = await tauriDaemonClient.fetchOrgPolicy();

    expect(invoke).toHaveBeenCalledWith("fetch_org_policy");
    expect(policy).not.toBeNull();
    expect(policy!.scope).not.toBeNull();
    expect(policy!.scope!.agentTypes).toEqual(["claude-code", "codex"]);
    expect(policy!.scope!.projectPathGlob).toBe("C:\\projects\\*");
    expect(policy!.policy.filesystem.readableRoots).toEqual(["C:\\projects"]);
    expect(policy!.policy.filesystem.writableRoots).toEqual(["C:\\projects\\out"]);
    expect(policy!.policy.filesystem.blockedRoots).toEqual(["C:\\Users\\dev\\.ssh"]);
    expect(policy!.policy.network.defaultAction).toBe("deny");
    expect(policy!.policy.network.allowedHosts).toEqual(["api.github.com"]);
    expect(policy!.policy.process.defaultAction).toBe("allow");
    expect(policy!.policy.process.allowedCommands).toEqual(["git", "npm"]);
  });

  it("maps loadProfile signature_status from Rust snake_case", async () => {
    invoke.mockResolvedValue({
      id: "claude-code.strict",
      displayName: "Claude Code Strict",
      detail: "Strict profile.",
      signature_status: "valid",
      filesystem: { readableRoots: [], writableRoots: [], blockedRoots: [] },
      network: { defaultAction: "deny", allowedHosts: [], blockedHosts: [] },
      process: { defaultAction: "allow", allowedCommands: [], blockedCommands: [] },
    });

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    const profile = await tauriDaemonClient.loadProfile("claude-code.strict");

    expect(invoke).toHaveBeenCalledWith("load_profile", { profileId: "claude-code.strict" });
    expect(profile.signatureStatus).toBe("valid");
    expect(profile.id).toBe("claude-code.strict");
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

  it("saveProfile passes profile to correct tauri command", async () => {
    invoke.mockResolvedValue(null);

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    const profile = {
      id: "claude-code.strict",
      displayName: "Claude Code Strict",
      detail: "Strict profile.",
      signatureStatus: "unsigned" as const,
      filesystem: { readableRoots: ["C:\\projects"], writableRoots: [], blockedRoots: [] },
      network: { defaultAction: "deny" as const, allowedHosts: [], blockedHosts: [] },
      process: { defaultAction: "allow" as const, allowedCommands: ["git"], blockedCommands: [] },
    };
    await tauriDaemonClient.saveProfile(profile);

    expect(invoke).toHaveBeenCalledWith("save_profile", { profile });
  });

  it("signProfile passes profileId and key to correct tauri command", async () => {
    invoke.mockResolvedValue(null);

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    await tauriDaemonClient.signProfile("claude-code.strict", "base64key==");

    expect(invoke).toHaveBeenCalledWith("sign_profile", { profileId: "claude-code.strict", signingKeyB64: "base64key==" });
  });

  it("loadRemoteProfile maps signature_status from Rust snake_case", async () => {
    invoke.mockResolvedValue({
      id: "remote.strict",
      displayName: "Remote Strict",
      detail: "Fetched remotely.",
      signature_status: "valid",
      filesystem: { readableRoots: [], writableRoots: [], blockedRoots: [] },
      network: { defaultAction: "deny", allowedHosts: [], blockedHosts: [] },
      process: { defaultAction: "allow", allowedCommands: [], blockedCommands: [] },
    });

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    const profile = await tauriDaemonClient.loadRemoteProfile("https://example.com/policy.json");

    expect(invoke).toHaveBeenCalledWith("load_remote_profile", { url: "https://example.com/policy.json" });
    expect(profile.signatureStatus).toBe("valid");
    expect(profile.id).toBe("remote.strict");
  });

  it("configureSync passes config to correct tauri command", async () => {
    invoke.mockResolvedValue(null);

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    const config = { endpointUrl: "https://audit.example.com", token: "tok-abc", stripPaths: true };
    await tauriDaemonClient.configureSync(config);

    expect(invoke).toHaveBeenCalledWith("configure_sync", { config });
  });

  it("syncAuditEvents maps snake_case SyncStatus response", async () => {
    invoke.mockResolvedValue({
      configured: true,
      queue_depth: 0,
      last_sync_at_ms: 1700001234000,
      last_error: "timeout",
    });

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    const status = await tauriDaemonClient.syncAuditEvents();

    expect(invoke).toHaveBeenCalledWith("sync_audit_events");
    expect(status.configured).toBe(true);
    expect(status.queueDepth).toBe(0);
    expect(status.lastSyncAtMs).toBe(1700001234000);
    expect(status.lastError).toBe("timeout");
  });

  it("configureOrgPolicyUrl passes url to correct tauri command", async () => {
    invoke.mockResolvedValue(null);

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    await tauriDaemonClient.configureOrgPolicyUrl("https://policy.example.com/org.json");

    expect(invoke).toHaveBeenCalledWith("configure_org_policy_url", { url: "https://policy.example.com/org.json" });
  });

  it("currentOrgPolicy returns null when no policy is cached", async () => {
    invoke.mockResolvedValue(null);

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    const policy = await tauriDaemonClient.currentOrgPolicy();

    expect(invoke).toHaveBeenCalledWith("current_org_policy");
    expect(policy).toBeNull();
  });

  it("currentOrgPolicy maps cached org policy snake_case fields to camelCase", async () => {
    invoke.mockResolvedValue({
      id: "cached-policy",
      name: "Cached Policy",
      description: null,
      scope: { agent_types: ["claude-code"], project_path_glob: null },
      policy: {
        filesystem: { readable_roots: ["C:\\projects"], writable_roots: [], blocked_roots: [] },
        network: { default_action: "allow", allowed_hosts: [], blocked_hosts: [] },
        process: { default_action: "allow", allowed_commands: [], blocked_commands: [] },
      },
      signature: null,
    });

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    const policy = await tauriDaemonClient.currentOrgPolicy();

    expect(policy).not.toBeNull();
    expect(policy!.id).toBe("cached-policy");
    expect(policy!.scope!.agentTypes).toEqual(["claude-code"]);
    expect(policy!.scope!.projectPathGlob).toBeNull();
    expect(policy!.policy.filesystem.readableRoots).toEqual(["C:\\projects"]);
  });
});
