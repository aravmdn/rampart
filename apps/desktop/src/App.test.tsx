import { cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import { afterEach } from "vitest";
import { describe, expect, it, vi } from "vitest";
import App from "./App";
import type { DaemonApi } from "./daemon/contracts";

afterEach(cleanup);

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
            terminalFirst: true,
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
      preflightCheck: vi.fn().mockResolvedValue({
        ready: true,
        diagnostics: [
          { severity: "pass", label: "Project directory", detail: "Exists." },
        ],
      }),
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
      loadProfile: vi.fn().mockResolvedValue(null),
      saveProfile: vi.fn().mockResolvedValue(undefined),
      listSessionHistory: vi.fn().mockResolvedValue([
        {
          session: {
            id: "session-prev",
            status: "stopped",
            profileId: "windows-safe",
            agentId: "codex",
            projectPath: "C:\\projects\\rampart",
            startedAtMs: 100,
            endedAtMs: 120,
          },
          events: [
            {
              id: "audit-prev",
              kind: "block_observed",
              message: "Blocked write to C:\\Users\\dev\\.ssh\\config",
            },
          ],
          violations: [
            {
              id: "violation-prev",
              operation: "write",
              target: "C:\\Users\\dev\\.ssh\\config",
              ruleId: "project_scope",
              message: "Policy denied write outside allowed project roots.",
            },
          ],
        },
      ]),
    };

    render(<App daemonClient={client} />);

    // Launcher view
    expect(await screen.findByText("Windows Safe")).toBeInTheDocument();
    expect(await screen.findByText("Recent History")).toBeInTheDocument();
    expect(screen.getByText("session-prev")).toBeInTheDocument();
    expect(screen.getByText(/Blocked write to C:\\Users\\dev\\.ssh\\config/)).toBeInTheDocument();
    expect(client.saveSelectedLaunchConfig).toHaveBeenCalledWith({
      projectPath: "C:\\projects\\rampart",
      agentId: "codex",
      profileId: "windows-safe",
    });

    // Preflight check was called
    expect(client.preflightCheck).toHaveBeenCalledWith(
      "C:\\projects\\rampart",
      "codex",
      "windows-safe",
    );

    // Terminal handoff note visible for terminal-first agent
    expect(await screen.findByText("Terminal Handoff")).toBeInTheDocument();

    // Preflight diagnostics visible
    expect(await screen.findByText("Preflight Diagnostics")).toBeInTheDocument();

    // Launch
    const launch = await screen.findByRole("button", { name: "Launch session" });
    fireEvent.click(launch);

    // Session console view
    expect(await screen.findByText("read blocked")).toBeInTheDocument();
    expect(
      screen.getByText(/Blocked read on C:\\Users\\dev\\.ssh\\config/),
    ).toBeInTheDocument();
  });

  it("navigates to history view and shows session detail", async () => {
    const client: DaemonApi = {
      loadLaunchContext: vi.fn().mockResolvedValue({
        projects: [{ id: "proj-1", label: "Project", path: "C:\\projects\\test", source: "detected" }],
        agents: [{ id: "claude-code", label: "Claude Code", detail: "Anthropic agent.", terminalFirst: true }],
        profiles: [{ id: "claude-code.standard", displayName: "Claude Code Standard", detail: "Standard profile." }],
        selected: { projectPath: "C:\\projects\\test", agentId: "claude-code", profileId: "claude-code.standard" },
        capabilities: { engineName: "mock", platform: "windows", capabilities: [] },
      }),
      saveSelectedLaunchConfig: vi.fn().mockResolvedValue(undefined),
      preflightCheck: vi.fn().mockResolvedValue({ ready: true, diagnostics: [] }),
      launchSession: vi.fn().mockResolvedValue({ id: "s1", status: "active", profileId: "claude-code.standard", agentId: "claude-code", projectPath: "C:\\projects\\test" }),
      stopSession: vi.fn().mockResolvedValue({ id: "s1", status: "stopped", profileId: "claude-code.standard", agentId: "claude-code", projectPath: "C:\\projects\\test" }),
      streamSessionEvents: vi.fn().mockResolvedValue({ audit: [], violations: [] }),
      loadProfile: vi.fn().mockResolvedValue(null),
      saveProfile: vi.fn().mockResolvedValue(undefined),
      listSessionHistory: vi.fn().mockResolvedValue([
        {
          session: {
            id: "hist-session-1",
            status: "stopped",
            profileId: "claude-code.standard",
            agentId: "claude-code",
            projectPath: "C:\\projects\\test",
            startedAtMs: 1700000000000,
            endedAtMs: 1700000030000,
          },
          capabilitySnapshot: null,
          events: [{ id: "e1", kind: "session_stopped", category: "session_lifecycle", message: "Session ended normally." }],
          violations: [
            {
              id: "v1",
              operation: "read",
              target: "C:\\Users\\dev\\.ssh\\id_rsa",
              ruleId: "fs.scope.blocked",
              ruleLabel: "Filesystem scope",
              message: "Read blocked outside project root.",
              platformNote: null,
              explanation: null,
            },
          ],
        },
      ]),
    };

    render(<App daemonClient={client} />);

    // Launcher loads and shows recent history
    expect(await screen.findByText("Recent History")).toBeInTheDocument();
    expect(screen.getByText("hist-session-1")).toBeInTheDocument();

    // Click "View all history"
    const viewAll = await screen.findByRole("button", { name: "View all history" });
    fireEvent.click(viewAll);

    // History view shows the session list
    expect(await screen.findByText("Session History")).toBeInTheDocument();
    expect(await screen.findByText("Rampart session history")).toBeInTheDocument();

    // Click "View" on the session
    const viewBtn = await screen.findByRole("button", { name: "View" });
    fireEvent.click(viewBtn);

    // Detail panel shows session info and violation
    expect(await screen.findByText("Session Detail")).toBeInTheDocument();
    expect(screen.getByText(/read blocked/)).toBeInTheDocument();

    // Back to history list
    const backToHistory = screen.getByRole("button", { name: "Back to history" });
    fireEvent.click(backToHistory);
    expect(screen.queryByText("Session Detail")).not.toBeInTheDocument();
  });
});
