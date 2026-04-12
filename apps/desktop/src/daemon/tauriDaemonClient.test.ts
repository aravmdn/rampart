import { beforeEach, describe, expect, it, vi } from "vitest";

const invoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke,
}));

describe("tauri daemon client", () => {
  beforeEach(() => {
    invoke.mockReset();
  });

  it("loads launch context through tauri invoke", async () => {
    invoke.mockResolvedValue({
      projects: [],
      agents: [],
      profiles: [],
      selected: {
        projectPath: null,
        agentId: null,
        profileId: null,
      },
      capabilities: {
        engineName: "greywall",
        platform: "windows",
        capabilities: [],
      },
    });

    const { tauriDaemonClient } = await import("./tauriDaemonClient");
    await tauriDaemonClient.loadLaunchContext();

    expect(invoke).toHaveBeenCalledWith("load_launch_context");
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
