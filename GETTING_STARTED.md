# Getting Started with Rampart

Rampart is a Windows-first desktop app that runs AI coding agents under enforced least privilege. It wraps agent execution in OS-level controls (Job Objects, Low Integrity token, WFP network filters) and surfaces live audit events, blocked actions, and session history without replacing the agent's native interface.

---

## Prerequisites

- **Windows 10 or Windows 11** — enforcement primitives (Job Objects, WFP, Low Integrity token) are Windows-only.
- **Administrator privileges** — required at runtime for WFP filter installation, SACL patching, and Job Object assignment. Without elevation, Rampart launches but enforcement falls back silently.
- **Node.js 20 or later** — `node --version` should print `v20.x.x` or higher.
- **pnpm 9.x** — install with `npm install -g pnpm@9` if not present.
- **Rust via rustup** — `rustup` with the `x86_64-pc-windows-msvc` target:
  ```
  rustup target add x86_64-pc-windows-msvc
  ```
- **Visual Studio 2022 Build Tools** — with the "Desktop development with C++" workload. This provides the MSVC linker (`link.exe`). Install via the [Visual Studio Installer](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022).
- **An AI agent CLI installed** — Rampart launches agents it can find on PATH. For example, `claude` (Claude Code), `codex`, `aider`, `goose`, `opencode`, or `gemini`. At least one must be present for a session to launch.

### Optional

- **WSL2** — enables the stronger WSL2 isolation mode. Rampart detects WSL2 at preflight and surfaces it as an opt-in option when available.

---

## Installation

Clone the repository and install JavaScript dependencies:

```powershell
git clone https://github.com/rampart-dev/rampart.git
cd rampart
pnpm install
```

`pnpm install` must complete before the desktop app or tests can run. Subsequent runs in a clean worktree also need it.

---

## First Launch

Open an **Administrator** PowerShell in the repository root, then run:

```powershell
pnpm dev:desktop
```

The first build compiles the Rust backend and may take 60–90 seconds. A Tauri window opens when ready.

If `cargo build` fails with `LNK1104`, the MSVC linker is not on PATH. Open a Developer Command Prompt (from the Visual Studio Installer) or confirm that `link.exe` is accessible. See [Troubleshooting](#troubleshooting) below.

---

## Quick Walkthrough

### 1. Pick a project

In the **Project picker** (left column), select a local directory. This becomes the project root for the session. The agent is granted write access to this directory; writes outside it are blocked by the Low Integrity token.

### 2. Pick an agent

In the **Agent picker**, select an installed agent such as Claude Code, Codex, or Aider. Rampart shows only agents it can find on PATH. Terminal-first agents (Claude Code, Codex, Aider) display a handoff note: Rampart controls launch and enforcement while you interact with the agent in its own terminal window.

### 3. Pick a profile

In the **Profile picker**, select a preset or a previously saved profile. Each agent ships with a **standard** profile (allows the agent's known API endpoints) and a **strict** profile (full network deny). Built-in presets are signed with an ed25519 key; the picker shows `[signed]` next to each.

### 4. Review preflight

The **Preflight Diagnostics** panel (right column) runs automatic checks: project directory exists, agent binary is on PATH, capability gaps, and org policy floor if configured. Items with `[x]` block launch. Fix them before proceeding.

The **Capability Snapshot** panel shows which enforcement primitives are active. When running as Administrator, Filesystem scope, Network egress, Process execution, and Session termination all show `supported`.

### 5. Launch

Click **Launch session**. The view switches to the session console. Status transitions from `launching` to `active`. For terminal-first agents, the agent's terminal window opens separately — interact with the agent there.

---

## During a Session

### Audit stream

The **Audit Stream** panel (bottom-right) polls every 3 seconds and shows structured events: file reads/writes, network connects, process spawns, and session lifecycle events.

### Violations

The **Violation View** panel (top-right) shows actions that were blocked. Each entry includes:

- What was blocked and why (policy rule match)
- A platform note if the block came from an OS primitive
- An **Adjust policy** button that opens the profile editor pre-populated with a targeted rule suggestion

Violations from WFP network blocks fire at the OS level and are surfaced in the Violation View as `network blocked` entries with a target of `port <N>`. The Adjust policy button on a WFP violation pre-populates `allowed_hosts` with that target — replace `port <N>` with the actual hostname before saving (WFP only carries port information at the kernel level).

### Profile editor

Click **Adjust policy** on a violation (or **Edit selected profile** from the launcher) to open the profile editor. Adjust filesystem paths, allowed hosts, and allowed commands in product language — no raw engine syntax. Saving returns you to wherever you came from (launcher or session console).

---

## Stopping and Reviewing History

Click **Stop session** in the session console. The agent process tree is terminated via the Job Object. Status changes to `stopped` and **Back to launcher** becomes enabled.

On the launcher, the **Recent History** panel shows up to three recent sessions. Click **View all history** to open the full history view, which shows per-session audit trails, violations, and capability snapshots.

---

## CLI Alternative

The headless `rampart` binary provides the same enforcement without the GUI:

```powershell
# Build
cargo build -p rampart-cli

# Run a session (preflight to stderr, JSONL events to stdout)
cargo run -p rampart-cli -- run `
  --agent claude-code `
  --profile claude-code.standard `
  --project C:\path\to\project

# Add --wsl2 to use the WSL2 isolation mode
cargo run -p rampart-cli -- run --agent claude-code --profile claude-code.strict --project . --wsl2

# List available profiles for an agent
cargo run -p rampart-cli -- list-profiles --agent claude-code --project .
```

The CLI applies all enforcement primitives (Job Object, Low Integrity token, WFP) through the same `RampartService` as the desktop. Run in an Administrator shell for full enforcement.

JSONL events are streamed to stdout. Each line is a JSON object with `kind`, `message`, and `timestamp` fields. Ctrl+C triggers a clean shutdown: session stops, audit queue is flushed, Job Object is released.

---

## Troubleshooting

### Not running as Administrator

**Symptom:** Capability Snapshot shows `unsupported` for Network egress or Filesystem scope. WFP filters are not installed.

**Fix:** Close the app and relaunch from an Administrator PowerShell. Rampart does not crash without elevation, but enforcement is disabled.

### Agent not on PATH

**Symptom:** Preflight shows a failed diagnostic: "Agent binary not found on PATH."

**Fix:** Install the agent CLI and confirm it is accessible in the same shell you used to launch Rampart. For Claude Code, `claude --version` should return without error. If using `scoop` or `winget`, ensure the install directory is in `$env:PATH`.

### MSVC linker missing (LNK1104)

**Symptom:** `pnpm dev:desktop` or `cargo build` fails with `LINK : fatal error LNK1104: cannot open file 'msvcrt.lib'`.

**Fix:**
1. Open the Visual Studio Installer.
2. Modify your Visual Studio 2022 Build Tools installation.
3. Ensure "Desktop development with C++" is checked.
4. After installation, open a **Developer PowerShell for VS 2022** (available from the Start menu) and re-run the build from there.

### WSL2 mode not appearing

**Symptom:** No WSL2 option in preflight or launcher, even though WSL2 is installed.

**Fix:** Rampart runs `wsl --status` at preflight to detect WSL2. Confirm that `wsl` is on PATH and that a default Linux distribution is set (`wsl --list --verbose`). If Rampart is launched from a non-standard shell that does not inherit `PATH`, WSL2 detection may fail.

### TypeScript tests failing

Run the desktop test suite to check for regressions:

```powershell
pnpm --filter @rampart/desktop exec vitest run
```

If `node_modules` is missing (e.g., after cloning into a fresh worktree), run `pnpm install` first.

---

## Known Limitations

- **WFP violation host names** — WFP fires at the kernel level and carries only the remote port, not the destination hostname. Network violations show `port <N>` as their target. The Adjust policy button still works; you'll edit the suggested entry to the actual hostname before saving.
- **WSL2 capability snapshot** — In WSL2 isolation mode, the capability snapshot still reflects Windows-native capabilities rather than WSL2-specific ones. Cosmetic; enforcement is not affected.
- **AppContainer isolation** — Reserved for a future release. The engine adapter layer leaves a clean seam for it.

---

## Where to Look Next

- [Architecture](docs/architecture.md) — module boundaries, IPC layer, engine adapter design
- [Threat model](docs/threat-model.md) — what Rampart contains and what it does not
- [Roadmap](docs/roadmap.md) — current phase status and what is coming next
- [Contributing](CONTRIBUTING.md) — how to run tests, submit changes, and find the issue tracker
