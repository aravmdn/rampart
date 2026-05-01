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
});
