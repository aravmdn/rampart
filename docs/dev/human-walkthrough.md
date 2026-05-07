# Rampart GUI walkthrough

Use this script to verify the desktop product end-to-end. It is the final manual step
after the automated audit run on 2026-05-06. Items tagged [VERIFIED BY AUTOMATION] have
already been validated programmatically — spot-check them only, do not re-run full tests.

---

## Prerequisites

- Repo at the post-audit state (after L4 doc edits and L2 engine-windows fixes).
- `pnpm install` already done. If not: run `pnpm install` from `C:\projects\rampart` first.
- Run as Administrator — Job Object, SACL, and WFP all require elevation. Non-elevated
  runs disable enforcement silently (enforcement falls back gracefully; no crash).
- Real agent binary installed. The ClaudeCode preset expects `claude.exe` on PATH or at
  `C:\Users\<you>\.local\bin\claude.exe`. Substitute any other installed agent if needed.
  Codex, Aider, Cursor, Windsurf, Copilot, Gemini, and Amp presets are also available.
- Scratch project directory ready. Create if missing:

      mkdir C:\projects\rampart\.rampart-walkthrough

  It can be empty. It is used as the sandboxed project root.

---

## How to start

PowerShell, from `C:\projects\rampart`, **as Administrator**:

    pnpm dev:desktop

Wait for the Tauri window to open. First build may take 60–90 seconds.

---

## Step 1 — App launches and shows launcher view

**Action:** Wait for the Tauri window. Do not interact yet.

**Look for:**
- Page title eyebrow reads "Rampart desktop shell".
- Heading reads "Launch console".
- Left column: "Project picker", "Agent picker", "Profile picker" sections visible.
- Right column: "Capability Snapshot" panel visible.
- No blank screen, no error toast, no JS console errors (right-click → Inspect to check).

**Pass:** All three picker sections and the Capability Snapshot panel render without errors.

**Fail:** If blank or stuck loading, open DevTools (right-click → Inspect → Console tab)
and look for a failed `invoke` call. A daemon start failure will surface there. Check that
you are running as Administrator and that the Tauri binary compiled cleanly.

---

## Step 2 — Project picker: select scratch directory

**Action:** In the "Project picker" section (left column, top), find the list of project
entries. Click the entry whose path matches `C:\projects\rampart\.rampart-walkthrough`
(or whichever scratch directory you created). If it does not appear, it needs to be
registered — check rampartd project registration; for now pick any existing project.

**Look for:**
- The selected item highlights (gains `choice-selected` styling — visually distinguished
  from unselected items).
- The path shown in the item's meta line matches the scratch directory.

**Pass:** Item highlights and remains highlighted after click.

**Fail:** If no projects appear at all, the daemon's `load_launch_context` call failed.
Check DevTools console for the IPC error.

---

## Step 3 — Agent picker: cycle through agents

**Action:** In the "Agent picker" section, click through each available agent option.
There should be 8: Claude Code, Codex, Aider, Cursor, GitHub Copilot CLI, Goose,
OpenCode, Gemini CLI (labels may differ slightly — these are the `agent.label`
values from the presets).

**Look for:**
- Each click selects that agent (highlighted).
- Agents marked as terminal-first (ClaudeCode, Codex, Aider) cause a "Terminal Handoff"
  panel to appear in the left column below the pickers, reading something like:
  "[Agent] is a terminal-first agent. Rampart will launch the session and apply
  enforcement, but interaction happens in the agent's own terminal."
- Non-terminal-first agents (Cursor, Windsurf, etc.) do not show that panel.

**Pass:** All 8 agents are selectable; terminal-first banner appears/disappears correctly.

**Fail:** If fewer than 8 agents appear, the `agent_profile_presets()` Rust function is
not returning all presets. Log the missing agent IDs.

**[VERIFIED BY AUTOMATION]** Per-agent preset existence for all 8 agents was validated in
L3 product loop tests. Spot-check here: confirm count is 8 and labels look correct.

---

## Step 4 — Profile picker: signed badge display

**Action:** In the "Profile picker" section, review the profile list. The description of
each item includes a badge prefix derived from `signatureStatus`.

**Look for:**
- Profiles signed by Rampart show `[signed]` at the start of their description text.
- Profiles with a tampered or invalid signature show `[signature invalid]` instead.
- Unsigned/user-created profiles show neither prefix (description starts directly).

**Pass:** At least the built-in presets show `[signed]` prefixes.

**Fail:** If all profiles show `[signature invalid]`, the signing keys are mismatched.
If all profiles show no prefix, signatures may not be set on presets — log as a regression.

**[VERIFIED BY AUTOMATION]** Profile sign/verify/tamper-detect round-trip was validated
in L3 `l3_validation` tests. Spot-check: confirm `[signed]` appears on at least one
preset; you do not need to test tamper detection here.

---

## Step 5 — Capability Snapshot panel (right column)

**Action:** Look at the "Capability Snapshot" panel in the right column. Do not click
anything.

**Look for:**
- Platform label (e.g. "Windows 11") and engine name (e.g. "engine-windows") shown under
  the heading.
- A list of capabilities: Filesystem scope, Network egress, Process execution,
  Violation streaming, Session termination.
- Each capability has a status badge: "supported" (info tone) or "unsupported" (warn tone).
- If any capability is "unsupported", a "Capability Warnings" panel appears below the
  snapshot reading: "Some enforcement capabilities are unsupported on this platform and
  engine combination. Policy rules in unsupported domains will not be enforced..."

**Pass:** All five capability rows render with a status badge. Unsupported items (if any)
are called out in the Capability Warnings panel.

**Fail:** If the panel is missing entirely, the `capabilities` field was not populated by
`load_launch_context`. Check the Rust daemon.

**[VERIFIED BY AUTOMATION]** Job Object, Low Integrity token, SACL, WFP, and ETW were
runtime-validated in L2 tests. The Capability Snapshot here reflects what the engine
reports — spot-check that "Network egress" and "Filesystem scope" show "supported" when
running as Administrator.

---

## Step 6 — Preflight Diagnostics panel

**Action:** With a project, agent, and profile all selected, look at the "Preflight
Diagnostics" panel in the right column (below Capability Snapshot).

**Look for:**
- A list of diagnostic items, each prefixed with ✓ (pass), ✗ (fail), or ⚠ (warn).
- Each item has a label and a detail line beneath it.
- If an org policy URL has been configured and fetched (Step 7 below), some items show
  an `[from org policy]` tag inline and the panel header shows an "Org policy floor active"
  status badge.
- If any diagnostic is `fail`, the "Launch session" button is disabled and a message
  reads "Preflight checks must pass before launch."

**Pass:** At least one diagnostic item renders. If all pass, "Launch session" button is
enabled.

**Fail:** If the diagnostics panel is missing, the `preflight_check` Tauri command is not
returning. Check DevTools console.

---

## Step 7 — Org Policy panel

**Action:** In the right column, scroll to the "Org Policy" panel. Enter a URL in the
"Policy URL" input (use a test endpoint or a local mock URL — even a 404 is fine for
verifying the UI flow). Click "Save URL", then click "Fetch policy".

**Look for:**
- After "Save URL": no visible change (config saved silently to daemon).
- After "Fetch policy": either the muted status line updates to "Active: [policy name]"
  if the fetch succeeds, or an error surfaces in DevTools if the URL is unreachable.
- If fetch succeeds and policy is active: the Preflight Diagnostics panel header gains
  an "Org policy floor active" badge, and any diagnostics originating from the org policy
  show `[from org policy]` inline.

**Pass (fetch succeeds):** "Active: ..." line appears; `[from org policy]` tags appear in
preflight list.

**Pass (fetch fails / placeholder URL):** No crash, no blank screen; error visible only
in DevTools console or as the "No org policy active." fallback text. The launcher remains
usable.

**Fail:** App crashes or the launcher becomes unusable on a bad URL.

**[VERIFIED BY AUTOMATION]** Org policy floor merge (strict-merge semantics) was validated
in L3 tests. Spot-check the GUI tags here — you do not need to verify the merge logic.

---

## Step 8 — Audit Sync panel

**Action:** In the right column, scroll to the "Audit Sync" panel. Fill in:
- Endpoint URL: any HTTPS URL (e.g. `https://example.com/api/v1/audit/events`)
- Bearer token: any string (e.g. `test-token`)
- Check the "Strip file paths before sending" checkbox.

Click "Save". Then click "Sync now".

**Look for:**
- After "Save": the muted status line at the bottom of the panel updates to something
  like "Configured · Queue: 0" (queue depth may vary).
- After "Sync now": the status line updates — either a last-sync timestamp appears
  ("Last sync: [datetime]") or an error is shown ("Error: ...") if the endpoint is
  unreachable.
- "Sync now" button is disabled before Save (when `syncStatus?.configured` is false).

**Pass:** Status line transitions from absent → "Configured · Queue: N" after Save.
"Sync now" becomes enabled. After Sync now, timestamp or error appears.

**Fail:** Status line never updates after Save — the `configure_sync` Tauri command is
not returning. Check DevTools.

**[VERIFIED BY AUTOMATION]** Sync queue depth, drain, and `strip_paths` reconfiguration
were validated in L3 tests. Spot-check: confirm the "Strip file paths before sending"
checkbox state persists through Save (status line reflects configured=true).

---

## Step 9 — Profile editor: open from launcher

**Action:** With a profile selected, click "Edit selected profile" (a secondary button
that appears below the "Profile picker" section when a profile is selected).

**Look for:**
- View transitions to the profile editor.
- Page eyebrow reads "Rampart profile editor"; heading reads "Edit profile".
- Three filesystem textareas: "Paths the agent can read (one per line)", "Paths the agent
  can write (one per line)", "Always blocked paths (one per line)".
- Network section: "Default network action" toggle row with "Block all by default" /
  "Allow all by default" buttons; "Allowed hosts" and "Always blocked hosts" textareas.
- Process execution section: same toggle + "Allowed commands" and "Always blocked commands"
  textareas.
- "Save profile" (primary) and "Cancel" (secondary) buttons at the bottom.

**Pass:** All three sections (Filesystem, Network, Process execution) render with their
textareas and toggles.

**Fail:** If the editor view does not appear, `loadProfile` IPC call failed. Check DevTools.

---

## Step 10 — Profile editor: edit and save

**Action:** In the editor, add a new path to the "Paths the agent can read" textarea
(e.g. `C:\projects\rampart\.rampart-walkthrough`). Click "Save profile".

**Look for:**
- View returns to the launcher (the `editorReturnView` is "launcher").
- The profile picker still shows the same profile selected.
- The profile's signature status in the picker description changes from `[signed]` to no
  prefix (saved profiles get `signatureStatus: "unsigned"` — the `[signed]` badge is
  dropped on save, which is correct behavior: user edits invalidate the preset signature).

**Pass:** Returned to launcher; `[signed]` prefix removed from the edited profile's
description text.

**Fail:** If view does not return, `saveProfile` IPC call failed. If `[signed]` is still
shown after save, the signature status is not being updated in state.

---

## Step 11 — Launch session

**Action:** Confirm a project, agent (use ClaudeCode or any installed agent), and profile
are all selected and preflight shows all passes. Click "Launch session".

**Look for:**
- Button is labelled "Launch session" and is enabled (not greyed out).
- On click: view immediately transitions to the session console.
- Page eyebrow reads "Rampart session console"; heading reads "Active session".
- "Session Status" panel (top-left) shows status badge transitioning: "launching" →
  "active" (badge tone: warn → info).
- Session panel shows: Session ID (a UUID-like string), Project path, Agent name,
  Profile name.
- "Stop session" button is visible and enabled once session ID is set.
- "Back to launcher" button is visible but disabled while status is "active" or
  "launching".

**Pass:** Session transitions to "active" status; Session ID is non-null; Stop button
enabled.

**Fail:** If status stays "launching" indefinitely, `launch_session` is hanging.
If status goes to "failed", the agent binary was not found — verify the agent is
installed on PATH. Check DevTools console for the error message.

---

## Step 12 — Session console: live event polling

**Action:** With the session active, wait up to 10 seconds without interacting.

**Look for:**
- "Audit Stream" panel (bottom-right): populated with audit event rows. Each row shows
  an event label (the `kind` field, e.g. `FileRead`, `ProcessSpawn`, `NetworkEgress`) and
  a message.
- Events refresh every 3 seconds (the poll interval in `App.tsx`).
- If the agent is generating activity, event count grows across polls.

**Pass:** At least one audit event appears within ~6 seconds of the session going "active".

**Fail:** If "No audit events yet." persists beyond 15 seconds and the agent is running,
the `stream_session_events` IPC call or the ETW audit pipeline is not delivering events.
This is a known gap: WFP/ETW end-to-end streaming in Windows Native mode was
pipeline-verified in automation but not driven end-to-end against a real agent making
real outbound calls. Log this as confirmation that the streaming end-to-end gap is still
open (see L2-T2 tracking item).

**[VERIFIED BY AUTOMATION]** ETW audit emission and session history record completeness
(audit events, violations, stop reason, capability snapshot) were validated in L2 and L3
tests. Spot-check: confirm event rows appear and have non-empty label and message fields.

---

## Step 13 — Trigger a blocked action (network block)

**Action:** Interact with the running agent (in its terminal if terminal-first) and
instruct it to make an outbound network request to a host not in the profile's allowed
list. A strict/default-deny profile blocks all outbound. For example, ask the agent to
`curl https://example.com` or fetch any URL.

**Look for:**
- "Violation View" panel (top-right in session console) updates from "No blocked actions
  yet." to a violation entry.
- Each violation entry shows:
  - Title: `[operation] blocked` (e.g. "network blocked")
  - Detail: explanation string from `explainViolation()`
  - Rule line: `Rule: [rule label] ([rule id])`
  - Platform note line (if present): `Platform note: ...`
  - "Adjust policy" button (for network/filesystem/process violations with a known target)

**Pass:** At least one violation appears in "Violation View" within ~6 seconds of the
blocked action.

**Fail:** If no violation appears after the block fires:
- If the agent's terminal shows a connection refused/blocked error but the GUI shows
  nothing, WFP is blocking at OS level but violation events are not streaming back to the
  console. This is the known end-to-end streaming gap (L2-T2). Log: "WFP blocks confirmed
  at OS level; violation not surfaced in GUI — streaming pipeline incomplete."
- If the agent's terminal shows no error (request succeeds), the WFP filter is not active.
  Verify you are running as Administrator.

**[VERIFIED BY AUTOMATION]** WFP per-app-ID outbound block filter installation (V4+V6)
was validated in L2 tests. The OS-level block fires. The uncertain part is GUI surfacing
of that block as a violation event — that is what this step is manually closing.

---

## Step 14 — Adjust policy from a violation

**Action:** In the "Violation View" panel, find a violation that has an "Adjust policy"
button (network, read, write, or execute violations will have one). Click "Adjust policy".

**Look for:**
- View transitions to the profile editor.
- A "Suggested rule change" panel appears at the top, above the main editor form.
  It reads something like: "The agent tried to reach '[host]' but network access was
  blocked. Add it to allow this host." with the specific value shown in a `<code>` block.
- The relevant textarea (e.g. "Allowed hosts") already contains the suggested value
  prepopulated.
- Page eyebrow reads "Rampart profile editor"; heading reads "Edit profile".

**Pass:** Editor opens pre-populated with the suggestion; "Suggested rule change" panel
is visible with the correct target value.

**Fail:** If editor opens but the suggestion panel is missing, `violationSuggestion()` did
not produce a suggestion for that violation type. Log the operation and target.

---

## Step 15 — Save adjusted policy and return to session

**Action:** In the profile editor (arrived from a session violation), click "Save profile"
without changing anything (or keep the prepopulated change).

**Look for:**
- View returns to the session console (not the launcher — `editorReturnView` is "session"
  when entered from a violation).
- The session is still running (status still "active").
- The violation that triggered the edit is still listed in "Violation View" (it is not
  cleared on policy save).

**Pass:** Returned to session console; session still active.

**Fail:** If returned to launcher instead of session, `editorReturnView` was not set
correctly when entering from a violation.

---

## Step 16 — Stop session

**Action:** Click "Stop session" in the session console (top-left, below Session Status).

**Look for:**
- Session status badge transitions from "active" → "stopped" (or "failed" if termination
  had an error).
- "Stop session" button remains visible (it is always rendered; disabled state depends on
  `session.id`).
- "Back to launcher" button becomes enabled (it is disabled while status is "active" or
  "launching").
- The "Recent History" panel on the launcher (visible after navigating back) should
  show the just-completed session.

**Pass:** Status badge changes to "stopped"; "Back to launcher" button enables.

**Fail:** If status does not change, `stop_session` IPC call is hanging. Check DevTools.

---

## Step 17 — Return to launcher and check Recent History

**Action:** Click "Back to launcher". On the launcher, scroll the right column to
"Recent History".

**Look for:**
- "Recent History" panel lists up to 3 most recent sessions.
- Each entry shows: session ID (bold), project path, first event message (if any),
  and the latest blocked action target and rule ID (if any violations occurred).
- A "View all history" button appears at the bottom of the list.

**Pass:** The session you just stopped appears at the top of the Recent History list.

**Fail:** If Recent History still shows "No persisted sessions yet.", the session was not
written to the persistence store by `stop_session`. Check rampartd persistence logic.

---

## Step 18 — History view: full list

**Action:** Click "View all history" in the Recent History panel (right column, launcher).

**Look for:**
- View transitions to history. Page eyebrow reads "Rampart session history"; heading reads
  "Session history".
- "Session History" panel (left column) lists all persisted sessions. Each row shows:
  started-at datetime (bold), duration, agent ID, profile ID, project path, event count,
  violation count, and a "View" button.
- A "Back to launcher" button is in the hero panel at the top.
- Right column shows "Select a session to inspect its events and violations." (placeholder).

**Pass:** At least one session row is visible; the session from Steps 11–16 is in the list.

**Fail:** Empty list after a confirmed completed session means persistence write failed.

**[VERIFIED BY AUTOMATION]** Session history record completeness (capability snapshot,
audit events, violations, stop reason) was validated in L3 tests.

---

## Step 19 — History detail: inspect a record

**Action:** Click "View" next to the session from this walkthrough.

**Look for:**
- Right column populates with "Session Detail" panel showing: session ID, Started,
  Duration, Agent, Profile, Project.
- Below Session Detail: "Violation View" panel listing all violations from that session.
  Each violation shows title, detail, rule label/ID, platform note (if any). No "Adjust
  policy" button here (that is session-console only).
- Below violations: "Audit Stream" panel listing all audit events from that session.
- A "Back to history" button in the Session Detail panel header.

**Pass:** Session Detail, Violation View, and Audit Stream all render with data from the
completed session.

**Fail:** If panels appear but are empty, the history entry was not persisted with events
and violations. Log which fields are missing.

**[VERIFIED BY AUTOMATION]** Violation explanation structure (rule_description +
platform_limitation + remediation_hint) was validated in L3 tests. Spot-check: confirm
violation detail text is human-readable (not a raw struct dump).

---

## Step 20 — Profile picker badge final check

**Action:** Click "Back to history" in Session Detail, then "Back to launcher" in the
history view hero. On the launcher, look at the "Profile picker" section.

**Look for:**
- Built-in preset profiles (ClaudeCode, Codex, Aider, etc.) show `[signed]` in their
  description prefix — unless you edited one in Step 10, in which case that one shows
  no prefix (unsigned).
- No profiles show `[signature invalid]` (that would indicate a signing key mismatch or
  intentional tamper test).

**Pass:** Unedited presets show `[signed]`; your edited profile from Step 10 shows no
prefix.

**Fail:** `[signature invalid]` on an unedited preset indicates a key rotation or signing
bug. Log which preset and the profile ID.

**[VERIFIED BY AUTOMATION]** Sign/verify/tamper-detect round-trip was validated in L3
`l3_validation` tests.

---

## WSL2 isolation mode (conditional — skip if WSL2 not installed)

**Action:** If WSL2 is installed on this machine, check whether the launcher's preflight
diagnostics include a WSL2 detection line. The preflight diagnostic label will reference
WSL2 availability.

**Look for:**
- A diagnostic row with a label referencing "WSL2" showing ✓ pass (if WSL2 is installed
  and detected) or ⚠ warn (if not installed).
- If a WSL2-capable profile is selected, the session should launch using
  `wsl --cd <linux_path> -- <cmd>` under the hood (not visible in the GUI directly,
  but the session goes "active" without Win32 enforcement errors).

**Pass:** WSL2 diagnostic row present in preflight; session launches normally.

**Fail:** WSL2 diagnostic missing entirely — `detect_wsl2()` may not be running during
preflight. Log as missing preflight item.

**[VERIFIED BY AUTOMATION]** WSL2 isolation mode and `win_path_to_wsl` conversion were
validated in L2 and L3 tests where WSL2 is present.

---

## When you're done

You have now manually verified:

1. App launch and all four views (launcher, session console, history, profile editor)
2. All three picker sections with correct selection behavior
3. Profile signature badge rendering (`[signed]` / `[signature invalid]` / no prefix)
4. Capability Snapshot and Capability Warnings panel
5. Preflight Diagnostics with org policy floor tags
6. Org Policy panel (URL save + fetch flow)
7. Audit Sync panel (save, configure, sync-now flow)
8. Profile editor (open from launcher, edit, save, return)
9. Session launch and status transitions
10. Live audit event polling (3s interval)
11. Violation surfacing in session console (or confirmed as streaming gap)
12. "Adjust policy" pre-population flow from a violation
13. Session stop and status transition
14. Recent History panel on launcher
15. History view list and "View all history" navigation
16. History detail panel (Session Detail, Violation View, Audit Stream)
17. WSL2 preflight diagnostic (if WSL2 present)

**If you found a bug, log it with:**

```
Surface: [which panel / button / view]
Step: [step number from this script]
Action taken: [exactly what you clicked or typed]
Expected: [what the script says should happen]
Actual: [what actually happened]
DevTools console output: [paste any IPC errors]
Automation status: [VERIFIED BY AUTOMATION / not verified]
```

**Known open gap (do not file as a bug unless behavior has changed):**
WFP blocks fire at OS level but violation events may not stream back to the session
console in Windows Native mode (Step 13). If the agent's terminal shows a blocked
connection but "Violation View" stays empty, this is the L2-T2 gap — confirm it is still
open and note it, but it is not a regression from the last audit.
