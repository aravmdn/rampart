import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import App from "./App";
import type { DaemonApi } from "./daemon/contracts";

describe("desktop shell", () => {
  it("shows blocked file read explanation after launch", async () => {
    const client: DaemonApi = {
      loadLaunchContext: vi.fn().mockResolvedValue({
        projects: [
          { id: "project-rampart", label: "Rampart", path: "C:\\projects\\rampart", source: "detected" },
        ],
        agents: [
          {
            id: "codex",
            label: "Codex",
            detail: "OpenAI coding agent in local desktop flow.",
          },
        ],
        profiles: [
          {
            id: "windows-safe",
            displayName: "Windows Safe",
            detail: "Project scoped read/write. Network denied by default.",
          },
        ],
        selected: {
          projectPath: "C:\\projects\\rampart",
          agentId: "codex",
          profileId: "windows-safe",
        },
        capabilities: {
          engineName: "greywall",
          platform: "windows",
          capabilities: [],
        },
      }),
      saveSelectedLaunchConfig: vi.fn().mockResolvedValue(undefined),
      launchSession: vi.fn().mockResolvedValue({
        id: "session-1",
        status: "active",
        profileId: "windows-safe",
        agentId: "codex",
        projectPath: "C:\\projects\\rampart",
      }),
      stopSession: vi.fn().mockResolvedValue({
        id: "session-1",
        status: "stopped",
        profileId: "windows-safe",
        agentId: "codex",
        projectPath: "C:\\projects\\rampart",
      }),
      streamSessionEvents: vi.fn().mockResolvedValue({
        audit: [
          {
            id: "audit-1",
            kind: "block_observed",
            message: "Blocked read outside selected project roots.",
          },
        ],
        violations: [
          {
            id: "violation-1",
            operation: "read",
            target: "C:\\Users\\dev\\.ssh\\config",
            ruleId: "project_scope",
            message: "Policy denied read outside allowed project roots.",
          },
        ],
      }),
      listSessionHistory: vi.fn().mockResolvedValue([]),
    };

    render(<App daemonClient={client} />);

    expect(await screen.findByText("Windows Safe")).toBeInTheDocument();
    expect(client.saveSelectedLaunchConfig).toHaveBeenCalledWith({
      projectPath: "C:\\projects\\rampart",
      agentId: "codex",
      profileId: "windows-safe",
    });
    const launch = await screen.findByRole("button", { name: "Launch session" });
    fireEvent.click(launch);

    expect(await screen.findByText("read blocked")).toBeInTheDocument();
    expect(
      screen.getByText(/Blocked read on C:\\Users\\dev\\.ssh\\config/),
    ).toBeInTheDocument();
    expect(screen.getByText("Capability Snapshot")).toBeInTheDocument();
  });
});
