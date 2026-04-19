# Phase 3 Runtime Test Plan

Verification of the end-to-end enforcement block flow. Requires Windows and admin rights.

## Prerequisites

- Windows 11, logged in as a user with admin rights (or ability to elevate)
- WSL2 installed (for WSL2 mode tests only)
- `pnpm dev:desktop` starts without error
- `claude` or `codex` CLI installed and on PATH

## Test 1 — Process containment (Job Objects)

**Goal**: verify that stopping a session kills the entire agent process tree.

1. Launch Rampart. Select any project, agent, and profile.
2. Start the session. Note the PID shown in the session console.
3. In a separate terminal: `tasklist /fi "pid eq <pid>"` — verify process is running.
4. In Rampart: click Stop session.
5. `tasklist /fi "pid eq <pid>"` — process should be gone.
6. Verify any child processes spawned by the agent are also gone.

**Pass**: all processes in the tree are terminated within ~1s of stopping the session.

## Test 2 — Filesystem write restriction (Low Integrity token)

**Goal**: verify the agent cannot write outside the project root.

1. Launch with a profile that has a narrow `writable_roots` (project root only).
2. Inside the agent session, attempt to write a file to `%USERPROFILE%\Desktop\test.txt`.
3. The write should be denied by the OS (Access Denied, not Rampart-intercepted).
4. Verify the agent *can* write a file inside the project root (e.g. `.\test-file.txt`).
5. Clean up: delete the test file.

**Pass**: write to Desktop denied; write to project root succeeds.

## Test 3 — Network enforcement (WFP filter)

**Goal**: verify outbound network connections are blocked for the agent process.

1. Launch with a profile whose network policy has `default_action: deny` and no `allowed_hosts`.
2. Inside the agent session, attempt an outbound HTTP/HTTPS request (e.g. `curl https://example.com`).
3. The connection should fail at the OS level (connection refused or timed out).
4. Check Rampart session console — a `NetworkBlocked` audit event should appear.

**Pass**: network request fails; audit event captured.

## Test 4 — ETW audit trail

**Goal**: verify ETW events are emitted during a session.

1. Before launching: start an ETW trace session:
   ```
   logman start RampartTrace -p {7E5A6B4C-F3D2-4A81-9B62-C1E0A4B8D7F6} -o rampart.etl -ets
   ```
2. Launch a session, trigger at least one block (see Test 3).
3. Stop the session.
4. Stop the trace: `logman stop RampartTrace -ets`
5. Inspect: `tracerpt rampart.etl -o rampart.csv -of CSV`
6. Verify events with `[rampart]` prefix appear in the CSV.

**Pass**: session-launched, block, and session-ended events visible in ETL output.

## Test 5 — WSL2 isolation mode

**Goal**: verify WSL2 mode launches the agent inside the Linux VM.

Requires: WSL2 installed with a distribution that has the agent CLI available (e.g. `claude`).

1. At preflight, verify "WSL2 isolation available" diagnostic appears as Pass.
2. Select WSL2 isolation mode (when the UI exposes it), or set `isolation_mode: "wsl2"` directly.
3. Launch the session.
4. In Task Manager, verify the process is `wsl.exe` or a WSL VM process, not a native Windows process.
5. Verify the working directory inside WSL maps correctly to the project root (e.g. `/mnt/c/projects/...`).

**Pass**: agent runs inside WSL2 VM; project root accessible at correct `/mnt/` path.

## Test 6 — End-to-end violation in UI

**Goal**: the full MVP loop — block occurs, violation appears in the session console with explanation.

1. Launch with a strict profile (deny network, narrow filesystem).
2. Let the agent attempt a blocked action (network request or write outside project root).
3. Verify the session console shows a `ViolationEvent` with:
   - correct `operation` type (network / write)
   - the blocked `target`
   - a `ViolationExplanation` with `ruleDescription` and `remediationHint`
4. Click "Adjust policy" on the violation — verify the profile editor opens pre-populated with a suggestion.

**Pass**: violation visible in UI with full explanation; policy refinement flow opens correctly.

## Known gaps (do not test as blocking)

- WSL paths on network drives or UNC paths are not handled by `win_path_to_wsl()`
- WFP filter attaches to the executable path; renamed/symlinked binaries may not match
- Low Integrity token is best-effort; restricted accounts may not get it applied
