# RAMPART — Project Reference Document
**Version:** 1.0 — April 2026  
**Purpose:** This document is the single source of truth for what Rampart is, why it exists, what it must do, and when. Every Codex agent session working on Rampart should read this file first. Do not deviate from architecture decisions without updating this file.

---

## 1. WHAT RAMPART IS

Rampart is a desktop application built Windows-first that sandboxes AI coding agents at the kernel level, enforcing filesystem, network, and process boundaries so agents like Claude Code, Cursor, Codex, and Aider can only access what you explicitly allow per project.

**One-sentence pitch:**  
Rampart is a firewall for AI coding agents — kernel-enforced, not prompt-enforced.

**The core problem it solves:**  
Every AI coding agent you run inherits your full operating system permissions. It can read your `.env` files, your SSH keys, your production credentials, and your entire home directory. It can make outbound network calls to any domain. When something goes wrong — a hallucinated command, a prompt injection, a runaway agent — most teams have no audit log of what was touched and no reliable way to stop it. 60% of organizations cannot terminate a misbehaving agent once it starts running.

**What Rampart does technically:**  
Rampart wraps kernel-level enforcement with a developer-friendly GUI, repository-friendly policy workflows, persistent audit logging, and a kill switch. Paid plans add a hosted team control plane for aggregated audit visibility, hosted policy coordination, alerts, and enterprise operating controls. `greywall` remains a reference implementation for macOS/Linux, but Windows-first delivery means Rampart cannot assume the same runtime is the primary engine on every platform. The enforcement layer is at the kernel or OS boundary — not a behavioral prompt, not a suggestion, and not a wrapper the agent can reason around.

---

## 2. MARKET CONTEXT AND GROWTH POTENTIAL

### The numbers that validate this product

- **88%** of organizations reported confirmed or suspected AI agent security incidents in the last 12 months (2026 survey, 900+ executives)
- **60%** of organizations cannot terminate a misbehaving AI agent once it starts running (Kiteworks 2026 Data Security Report)
- **82%** of MCP servers tested in 2025 were vulnerable to path traversal attacks due to unscoped filesystem permissions
- **80.9%** of technical teams have moved past planning into active deployment of AI agents (2026)
- **$3.6 billion** raised by the top 10 agentic AI security startups as of April 2026
- **$392 million** in agentic AI security funding announced in the two weeks surrounding RSAC 2026 alone
- **$172 billion** — projected AI cybersecurity market by 2029 (Gartner, 73.9% CAGR)
- **40%** of enterprise applications are projected to embed task-specific AI agents by end of 2026 (Gartner)
- **$9 billion+** — global agentic AI market size in 2026, growing rapidly

### Why the developer-tool layer is specifically underserved

Enterprise security vendors (Oasis Security at $120M Series B, Zenity, WitnessAI, CyberArk) are building for Fortune 500 CISOs with six-figure contract values. They are not building for the engineering manager at a 30-person company whose team runs Claude Code daily and has no policy whatsoever for agent permissions.

Greywall (GreyhavenHQ/greywall) is the best open-source implementation of kernel-level agent sandboxing. It shipped v0.3.0 on April 1, 2026. It has ~109 GitHub stars and is maintained by a small company using it in their own production. It has no GUI, no team management, no audit log, no kill switch, and no managed product layer. That is precisely what Rampart builds.

### Growth trajectory for a product like this

- **Months 1–3:** Developer adoption via HN, OSS community, Reddit. Target: 300–500 installs, 5–10 paying teams.
- **Months 4–6:** Team features unlock engineering manager buyers. GitHub Action distribution. Target: 1,000+ installs, $2,000–$5,000 MRR.
- **Months 7–12:** Enterprise pilots via direct outreach. Content/SEO compounds. SOC2 audit in progress. Target: $10,000–$30,000 MRR, first enterprise contract at $500–$2,000/month.
- **Year 2:** broader platform support beyond the initial Windows-first release, CI-first enterprise offering, possible pre-seed raise ($500K–$1M is realistic with traction). Comparable OSS-led devtools like GitGuardian, Snyk, and Sourcegraph all scaled through this exact path.

---

## 3. ARCHITECTURE DECISIONS — DO NOT CHANGE WITHOUT UPDATING THIS FILE

| Decision | Choice | Reason |
|---|---|---|
| Desktop framework | Tauri (Rust + TypeScript) | Smaller binary, better performance, native OS integration for sandboxing hooks |
| Frontend language | TypeScript + React | Speed of development for UI components |
| Backend language | Rust (via Tauri) | Required for greywall binary integration and low-level process management |
| Sandboxing engine | greywall binary (bundled) | Apache 2.0, proven kernel-level enforcement, covers Linux + macOS |
| greywall integration | Wrap binary via `std::process::Command` | Do not import as a library — keep greywall updates independent of Rampart releases |
| Local storage | SQLite via `tauri-plugin-sql` | Session logs, profiles, settings — all local, no cloud required for free tier |
| Team backend | Fly.io (single instance, Postgres) | Optional hosted control plane for team and enterprise features |
| License | Apache 2.0 | Matches greywall license, enables enterprise use, builds developer trust |
| Platforms | Windows first, macOS and Linux following | Windows is the primary product constraint and must shape the execution model |
| macOS/Linux | Follow after the Windows execution model is solid | `greywall` remains the reference adapter path on those platforms |

---

## 4. FEATURE SPECIFICATION — ALL PHASES

### Phase 1 — Core (Weeks 3–5): The MVP that makes greywall usable

**F1.1 — One-click sandboxed session launch**
- Detect installed agents on the system (claude, cursor, codex, aider, goose, gemini)
- Generate correct greywall command for detected agent + current directory
- Launch agent through sandbox with one button click
- Display: active session indicator, agent name, elapsed time, sandbox status

**F1.2 — Real-time session view**
- Parse greywall eBPF violation logs as they stream
- Render live file tree: allowed operations (green), blocked (red), pending (amber)
- Network connection feed (Linux only — greyproxy): domains contacted, domains blocked
- Blocked action counter: "Agent attempted 3 blocked operations this session"
- End session button: kills sandbox, generates session summary

**F1.3 — Project profiles (zero-config)**
- 8 preset templates: Next.js, Python ML, Go service, Ruby on Rails, monorepo, data science, general web, bare minimum
- Auto-detect project type from working directory (package.json → Next.js, pyproject.toml → Python ML, go.mod → Go, etc.)
- Auto-suggest appropriate template on first launch in a directory
- Custom rule editor: path field + allow/deny toggle — no raw YAML ever exposed to user
- Persist chosen profile per directory in `~/.config/rampart/projects.json`

### Phase 2 — Beta launch (Weeks 6–8): Ship and get users

**F2.1 — Installer and distribution**
- Homebrew cask: `brew install --cask rampart`
- `.deb` package for Linux (via GitHub Releases)
- Auto-updater (Tauri built-in)
- First-run onboarding flow: 3 screens maximum

**F2.2 — Stripe billing integration**
- Free tier: full local sandboxing, local audit history, repository-backed policy files, no required cloud
- Team tier ($15/seat/month): hosted audit aggregation, hosted policy coordination, remote alerts, manager visibility
- Stripe Checkout embedded — no custom billing page needed initially

### Phase 3 — Team features (Weeks 9–12): The features managers pay for

**F3.1 — Persistent audit log**
- SQLite schema: `sessions` table with session_id, timestamp, agent_name, project_dir, files_read[], files_written[], files_blocked[], network_domains[], network_blocked[], duration_seconds, blocked_action_count
- Session history view: list with quick stats, expandable to full file access tree
- Export: JSON and CSV
- Search: full-text search across all session logs ("show every session where agent touched ~/.ssh")
- Retention: configurable, default 90 days

**F3.2 — Kill switch**
- System tray "Emergency Stop" button
- Terminates all active sandboxed agent processes via SIGKILL
- Logs the termination event with reason
- Must work even if main app window is closed
- Test requirement: must kill all processes within 500ms

**F3.3 — Configurable alerts (local)**
- "Notify me if agent accesses any path outside /src" — OS desktop notification
- Suspicious activity threshold: if blocked action count exceeds N in a session, prompt user to terminate
- Alert log: all triggered alerts stored in SQLite

**F3.4 — Repository policy workflows and team coordination**
- Export: serialize all project profiles to `.rampart/policy.json` in working directory
- Import: read `.rampart/policy.json` on app startup, apply profiles automatically
- Policy-as-code: commit `.rampart/policy.json` to git → whole team gets the same baseline policy on pull
- Hosted team features build on top of this baseline with aggregated session view, manager visibility, and hosted coordination flows

### Phase 4 — Distribution growth (Weeks 13–16): GitHub Action + content

**F4.1 — Linux headless / CI mode**
- CLI: `rampart run --profile nextjs --headless -- claude [command]`
- No GUI dependency — pure CLI for CI/CD environments
- Exit code: 0 if session completed within policy, 1 if policy violations exceeded threshold
- JSON output of session summary to stdout for CI log parsing

**F4.2 — GitHub Action**
- Published as `rampartHQ/sandbox-action@v1` to GitHub Actions Marketplace
- Usage: wraps any agent command in a Rampart sandbox within a workflow step
- Input parameters: `agent`, `profile`, `project-dir`, `max-violations`
- Output: session-summary JSON artifact

### Phase 5 — Enterprise foundation (Weeks 17–20): Compliance and first contracts

**F5.1 — SSO**
- Google Workspace via OAuth 2.0
- Okta via SAML
- Required by enterprise IT procurement

**F5.2 — Data residency**
- Team dashboard deployable to EU region (Fly.io Frankfurt) or US region
- User selects region on team signup — cannot be changed after

**F5.3 — SOC2 readiness**
- Vanta or Drata integration for evidence collection
- All infrastructure access logged
- Audit trail for every data access in the web dashboard

---

## 5. WHAT RAMPART DOES NOT DO — IMPORTANT CONSTRAINTS

- Does not intercept or modify agent prompts or completions (that is a different product)
- Does not provide model-level guardrails or content filtering
- Does not yet claim cross-platform parity; Windows, macOS, and Linux may require different enforcement runtimes and capabilities
- Does not sandbox agents running inside Docker containers (greywall operates on host processes)
- Does not provide network filtering on macOS in v1 (greyproxy transparent proxying is Linux-only — macOS gets env-var-based SOCKS5 only)
- Does not store any code, prompts, or completions in the cloud — audit logs contain only filesystem paths, network domains, and metadata

---

## 6. COMPETITIVE LANDSCAPE — HOW TO POSITION

| Competitor | What they do | Why Rampart is different |
|---|---|---|
| greywall (OSS) | Kernel sandboxing, CLI only | Rampart is the managed product layer — GUI, team features, audit log, kill switch |
| Oasis Security | Enterprise non-human identity governance | Fortune 500 buyer, six-figure contracts, not developer tooling |
| Docker MCP Gateway | MCP-specific security interceptors | Rampart works across all agents, not just MCP-connected tools |
| CyberArk AI Defense | Enterprise identity for agents | Same as Oasis — wrong market segment for v1 |
| .cursorrules / CLAUDE.md | Behavioral (prompt-level) constraints | Behavioral ≠ enforcement. Agent can reason around a prompt. Rampart is kernel-level. |

**Positioning statement:**  
Rampart is for engineering teams, not enterprise security teams. It solves the problem that exists today — agents running with unconstrained permissions on developer machines — not the theoretical enterprise governance problem that well-funded startups are solving for CISOs.

---

## 7. KNOWN TECHNICAL RISKS AND MITIGATIONS

**Risk 1: Landlock atomic write limitation**
- Problem: Agents often write to a temp path then rename into place. Landlock evaluates paths at syscall time — the rename from `/tmp/agent-edit-xyz` to `/src/index.ts` involves a path outside the declared allow list.
- Mitigation: Always scope profiles to directory level, never file level. Document this limitation clearly. Watch greywall issue #62 for upstream fix.

**Risk 2: macOS network monitoring gap**
- Problem: Greyproxy transparent network capture requires a TUN device — Linux-only. macOS gets SOCKS5 via environment variables, which requires the agent to respect the proxy config.
- Mitigation: Ship macOS with "network monitoring: limited" status indicator. Full network observability is a Linux-only feature in v1. Do not hide this from users.

**Risk 3: greywall upstream dependency**
- Problem: Greyhaven is a small company. If they abandon greywall or change the license, Rampart's engine is affected.
- Mitigation: Maintain a Rampart fork of greywall from day one. Contribute patches upstream (goodwill + keep fork minimal). The fork gives us independence if needed.

**Risk 4: Platform vendors shipping native sandboxing**
- Problem: Anthropic, Cursor, or GitHub could ship built-in agent sandboxing as a feature.
- Mitigation: Rampart's moat is not the sandboxing — it's the team policy management, audit log, and kill switch across multiple agents simultaneously. No vendor will ship a cross-agent, cross-team policy product as a free feature.

**Risk 5: Developer trust for a security tool**
- Problem: Developers are suspicious of security tools that sit between them and their filesystem.
- Mitigation: Apache 2.0 license, open source, no telemetry in free tier, explicitly documented data handling. The OSS nature is non-negotiable.

---

## 8. DISTRIBUTION STRATEGY

**Primary channels (in order of priority):**

1. **Hacker News Show HN** — post when v0.1.0 ships. Lead with the demo (agent reads .env → live damage → sandboxed session → agent adapts). Technical HN audience = early adopters who will file issues and spread via word of mouth.

2. **GitHub** — the repo itself is a distribution channel. Every star, every issue, every fork is discoverability. Respond to every issue within 24 hours in the first 3 months.

3. **Reddit** — r/netsec, r/ClaudeAI, r/cursor_ai, r/devops. Different framing per community. Security angle for r/netsec, practical dev angle for r/ClaudeAI.

4. **LinkedIn** — engineering managers live here. Post the problem statement, not the product. "60% of orgs can't kill a misbehaving agent" is a headline that resonates with the buyer persona.

5. **Security newsletters** — tl;dr sec (3.5M subscribers), TLDR DevOps, The New Stack. Pitch after v0.1.0 ships. The "29M secrets leaked on GitHub, AI-assisted commits leak at 2x rate" angle is a ready-made news hook.

6. **GitHub Action marketplace** — publishing `rampartHQ/sandbox-action@v1` creates a second discovery surface. Every repo that uses it is a reference installation.

7. **Content/SEO** — "What can Claude Code access on your machine?" ranks for a query developers are actively searching. Write 3–4 technical posts in Phase 4.

---

## 9. MONETIZATION

**Free tier (individual, always free):**
- Full local sandboxing for one user
- All 8 project profile templates
- Real-time session view
- Local session history (last 30 days)
- Repository-backed policy import/export via `.rampart/policy.json`
- No cloud connectivity, no account required

**Team tier ($15/seat/month, min 3 seats = $45/month minimum):**
- Everything in free tier
- Web dashboard: aggregated sessions across team
- Hosted policy coordination, review, and distribution
- Kill switch with remote trigger from dashboard
- Audit log export (JSON, CSV)
- 90-day session history
- Email alerts for policy violations

**Enterprise (custom pricing, starts ~$500/month for 20+ seats):**
- Everything in team tier
- SSO (Google Workspace, Okta)
- SOC2 Type II report
- Custom data retention policy
- EU data residency option
- Dedicated Slack channel for support
- SLA: 99.9% uptime on dashboard, 4-hour response on critical issues
- MSA + DPA on request

---

## 10. WEEK-BY-WEEK TIMELINE

This is the authoritative build sequence. Each week has a goal, specific tasks, and a defined output. Do not start a week's tasks until the previous week's output is complete and tested. When using Codex to build, scope each session to the specific tasks listed — do not ask Codex to build beyond the current week.

---

### PRE-BUILD — Days 1–14 (Before Week 1 of coding)

**Goal:** Validate the idea with real people before writing code. Set up infrastructure. Understand greywall internals.

**Day 1–3:**
- Install greywall v0.3.0 on your primary machine (Linux preferred)
- Run `greywall -- claude`, `greywall -- cursor`, observe each profile in action
- Deliberately trigger a blocked action: attempt to read `~/.ssh/id_rsa` during a session
- Run `greywall --learning -- claude` and read the generated YAML profile completely
- Read greywall source files: `linux.go` (bubblewrap), `linux_landlock.go` (Landlock), `macos.go` (Seatbelt), `greyproxy` architecture
- Document: every friction point you personally experience as a user. This list becomes your UX backlog.

**Day 4–7:**
- Post LinkedIn (Option B: "60% of orgs can't kill a misbehaving agent" hook — see Section 8)
- Post in greywall GitHub Discussions: introduce yourself, say you're building a GUI layer, ask for feedback
- Post in HN, r/ClaudeAI, r/cursor_ai: "researching AI agent permissions — 15 min call?"
- Complete 15–20 customer discovery calls using exactly 4 questions:
  1. Do you know what files your agent can access right now?
  2. Has an agent ever done something unexpected or touched something it shouldn't?
  3. Does your company have any policy for what agents are allowed to do?
  4. Would you pay $15/seat/month for a tool that sandboxed agent access and gave you an audit log?
- If fewer than 8 of 20 people say yes to question 4: reassess the positioning, not the product
- Record call notes. Identify the 3 most common pain points — these become v1 feature priorities

**Day 8–14:**
- Register domain (rampart.dev or equivalent)
- Create GitHub org `rampartHQ`, create repo `rampart`, set to public, add Apache 2.0 license
- Write `README.md`: one-sentence description, demo GIF placeholder, "Why this exists" section with 3 key stats, "How it works" paragraph, "Status: Early development", "Star to follow progress"
- Write `CLAUDE.md` (this file, or a shorter version of it) — put it in the repo root
- Write `ROADMAP.md`: Phase 1–5 features as a public-facing list (use the feature spec from Section 4)
- Create issue templates: `bug_report.md`, `feature_request.md`
- Set up GitHub Actions CI (lint + build check only — no tests yet)
- Create dead-simple landing page: one paragraph, one email capture. Use Carrd ($19/year) or GitHub Pages. Do not spend more than 2 hours on this.
- Create Stripe account. Set up team tier product ($15/seat/month). Do not integrate yet — just have it ready.

**Output checkpoint:** Repo is public with README, CLAUDE.md, ROADMAP.md. Landing page is live. 15+ discovery calls done. At least 8 people said yes to paying. You understand greywall source code well enough to explain the Landlock atomic write limitation.

---

### WEEK 1 — Desktop app shell + greywall binary integration

**Goal:** Working app that launches a sandboxed agent with one click. No YAML. No terminal required.

**Tasks:**
- Scaffold Tauri project: `npm create tauri-app@latest rampart -- --template react-ts`
- Set up project structure: `src/` (frontend), `src-tauri/` (Rust backend), `src-tauri/binaries/` (bundled greywall binary)
- Bundle greywall binary: download greywall release binary for the target platform, include in `src-tauri/binaries/`, configure Tauri `externalBin` in `tauri.conf.json`
- Rust backend — implement `detect_agents()`: scan `$PATH` for known agent binaries (claude, cursor, codex, aider, goose, gemini-cli). Return list of installed agents.
- Rust backend — implement `build_greywall_command(agent: &str, project_dir: &str) -> Result<Command>`: look up the correct greywall profile for the given agent, construct the full `greywall --profile [agent] -- [agent_binary]` command. Error cases: greywall binary not found (return specific error with install hint), invalid directory, unknown agent.
- Rust backend — implement `launch_sandboxed_session(agent: &str, project_dir: &str) -> Result<SessionHandle>`: calls `build_greywall_command`, spawns the process, returns PID and session start time.
- Frontend — main window: "Rampart" header, project directory picker (native file dialog via Tauri), agent selector (dropdown of detected agents), "Start sandboxed session" button
- Frontend — session status indicator: "No active session" / "Session active: claude — 3m 24s"
- System tray: icon, menu items: "Start session", "View history" (disabled for now), "Settings" (disabled), "Quit"
- Error handling: if greywall not found, show banner "greywall not installed — install with: brew install greywall" with copy button

**Codex prompt for this week:**  
> "We are building Rampart, a Tauri desktop app that wraps the greywall binary. Week 1 task: [paste specific task from list above]. Follow the architecture in CLAUDE.md. The greywall binary is at `src-tauri/binaries/greywall`. Do not implement any week 2+ features."

**Output checkpoint:** App builds and runs on your machine. Clicking "Start sandboxed session" launches Claude Code (or another agent) through greywall. The session is visibly sandboxed — verify by attempting to access `~/.ssh` during the session.

---

### WEEK 2 — Real-time session view

**Goal:** Live dashboard showing what the agent is touching as it runs.

**Tasks:**
- Rust backend — implement log parser: tail greywall's eBPF violation output in real time. Parse each line into a structured event: `{ timestamp, event_type: "allow"|"block", path, operation: "read"|"write"|"exec", pid }`
- Use Tauri's event system (`emit` from Rust → `listen` in TypeScript frontend) to stream parsed events to the frontend in real time
- Frontend — file tree component: render a live tree of filesystem operations. Group by directory. Color code: green dot for allowed, red dot for blocked, count of operations per path
- Frontend — network feed (Linux only): parse greyproxy output if available, display a scrolling list of domain connections with allow/block status. If not on Linux, show "Network monitoring available on Linux" message — do not hide this.
- Frontend — stats bar: "Files read: 23 | Files written: 7 | Blocked: 3 | Network calls: 12"
- Frontend — "End session" button: sends SIGTERM to the sandboxed agent process, waits 2 seconds, sends SIGKILL if still running, generates session summary modal
- Session summary modal: duration, files read count, files written count, blocked attempts count, network domains list, one-line assessment ("Session completed within policy" or "Session triggered 3 policy violations")
- SQLite — create sessions table and write session summary on end

**Output checkpoint:** During a live session, you can see in real time which files the agent is reading and which operations are being blocked. Ending the session generates a summary that gets saved locally.

---

### WEEK 3 — Project profiles

**Goal:** Zero-config setup. Developer picks a project type and is sandboxed correctly without knowing anything about Landlock or YAML.

**Tasks:**
- Define 8 profile templates as TypeScript constants (not YAML — keep them in code): `nextjs`, `python_ml`, `go_service`, `ruby_rails`, `monorepo`, `data_science`, `general_web`, `bare_minimum`. Each template is an object: `{ name, description, allowRead: string[], allowWrite: string[], denyRead: string[], networkAllow: string[], networkDeny: string[] }`
- Auto-detection logic: scan working directory for `package.json` + `next.config.*` → nextjs; `pyproject.toml` or `requirements.txt` → python_ml; `go.mod` → go_service; `Gemfile` → ruby_rails; multiple package managers at root → monorepo
- Profile picker screen: card grid of 8 templates, description on each, auto-detected profile highlighted, "Use this profile" button
- Custom rule editor (advanced): simple list of path entries, each with read/write toggles and allow/deny. No raw YAML ever shown.
- Persistence: save chosen profile per project directory to `~/.config/rampart/projects.json` as `{ [absolutePath]: profileName }`
- "Use this profile for all future sessions here" checkbox — checked by default
- On session start: load saved profile for current directory automatically, skip picker if profile is set

**Output checkpoint:** Open a Next.js project directory → app auto-detects and suggests the Next.js profile → one click to start → session is sandboxed with appropriate rules → profile is remembered for next time.

---

### WEEK 4 — Polish and installer prep

**Goal:** v0.1.0 is stable, installable, and has zero known critical bugs. Ready to ship to strangers.

**Tasks:**
- Installer experience: configure Tauri bundler for `.dmg` (macOS) and `.deb` (Linux). Test both installers on a clean machine (use a VM for Linux).
- Homebrew: create `rampartHQ/homebrew-rampart` repo with a Cask formula pointing to the `.dmg` GitHub Release
- First-run onboarding: 3-screen wizard. Screen 1: "What Rampart does" + the 2 key stats. Screen 2: "Install greywall" with auto-detect + install button. Screen 3: "Pick your first project". Maximum 3 clicks to first sandboxed session.
- Auto-updater: configure Tauri updater to check GitHub Releases on startup. Prompt user when update available — do not auto-install silently.
- Error messages: audit every error state. Every error message must tell the user: what went wrong, why, and exactly what to do next. No "Unknown error" messages.
- Manual QA checklist: run through 10 scenarios including: first install, greywall not installed, invalid project directory, agent not found, blocked action during session, session crash, end session, update available, profile save, profile load.
- Write `CHANGELOG.md` with v0.1.0 entry. Every user-visible change listed. Engineering managers read changelogs.
- Record the demo video: 90 seconds. Scene 1 (30s): run Claude Code without Rampart — agent reads `.env`, hits live Stripe API. Scene 2 (30s): same session with Rampart — agent says "I couldn't access .env, using placeholders". Scene 3 (30s): show session audit log of blocked operations. Upload to YouTube (unlisted for now) and embed in README.

**Output checkpoint:** A stranger can install Rampart from Homebrew, complete onboarding in under 2 minutes, run their first sandboxed session, and end it — all without touching a terminal or reading documentation.

---

### WEEK 5 — Beta launch

**Goal:** v0.1.0 is public. First 100 installs. First paying team.

**Day 1 of week 5 (Tuesday):**
- Post "Show HN: Rampart – kernel-level sandbox for AI coding agents (Claude Code, Cursor, Codex)" at 9am EST
- HN post structure: what it does (2 sentences), the demo video link, the key stat ("60% of orgs can't terminate a misbehaving agent"), how it works technically (kernel-level, not prompt-level), link to GitHub repo, link to install
- Monitor HN for 4 hours. Respond to every comment within 30 minutes. This directly affects ranking. Be direct, technical, honest about limitations (macOS network gap, no Windows).

**Same day:**
- Post to r/netsec: security framing ("AI agents run with your full permissions — here's a kernel-level fix")
- Post to r/ClaudeAI and r/cursor_ai: practical framing ("I built a sandbox for Claude Code so it can't touch my SSH keys")
- Email waitlist from landing page

**Days 2–3:**
- Product Hunt launch (secondary — do not coordinate with HN, different audience)
- Pitch email to tl;dr sec, TLDR DevOps, The New Stack: 3 sentences + demo video link. Subject: "Kernel-level sandbox for AI coding agents — launching today"

**Days 4–7:**
- Read every GitHub issue, every comment, every email. Categorize only: bugs (fix this week), UX confusion (fix this week), feature requests (log for later, do not build)
- Ship v0.1.1 within 72 hours with top 3 bug fixes
- Enable Stripe team tier — manually reach out to the 8+ people from discovery calls who said yes to paying. Offer 30-day free trial in exchange for a 30-minute feedback call.

**Output checkpoint:** v0.1.0 live, 100+ GitHub stars, 50+ installs, 5+ paying teams, Discord server created with 20+ members.

---

### WEEK 6 — Persistent audit log

**Goal:** A developer can answer "what did my agent touch in the last 7 days?" in under 30 seconds.

**Tasks:**
- Finalize SQLite schema:
  ```sql
  CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    started_at INTEGER NOT NULL,
    ended_at INTEGER,
    agent_name TEXT NOT NULL,
    project_dir TEXT NOT NULL,
    profile_name TEXT NOT NULL,
    files_read TEXT NOT NULL DEFAULT '[]',  -- JSON array of paths
    files_written TEXT NOT NULL DEFAULT '[]',
    files_blocked TEXT NOT NULL DEFAULT '[]',
    network_allowed TEXT NOT NULL DEFAULT '[]',  -- JSON array of domains
    network_blocked TEXT NOT NULL DEFAULT '[]',
    blocked_action_count INTEGER NOT NULL DEFAULT 0,
    terminated_by_user INTEGER NOT NULL DEFAULT 0
  );
  ```
- Session history view: list of past sessions, newest first. Each row shows: date, agent, project name (basename of dir), duration, blocked count, expand arrow
- Expanded session view: full file tree of operations during that session, network domain list
- Export: "Export as JSON" and "Export as CSV" buttons, saves to Downloads folder via native save dialog
- Search: text input at top of history view. Filters sessions where any path in files_read/written/blocked contains the search term. Useful query: user types "~/.ssh" → sees all sessions where agent touched SSH directory.
- Local retention setting in preferences: 30 days (default), 90 days, 1 year, forever

**Output checkpoint:** Run 5 sessions across 3 projects. Open session history. Find a specific blocked operation. Export the log as CSV. Verify the data is accurate.

---

### WEEK 7 — Kill switch + alerts

**Goal:** Any active sandboxed session can be terminated in under 500ms from the system tray. Suspicious sessions trigger a desktop notification.

**Tasks:**
- Kill switch implementation: system tray menu item "Stop all agent sessions" → sends SIGKILL to all tracked PIDs → verifies processes are dead within 500ms → logs termination event to SQLite
- Kill switch must work when main app window is closed — the tray process must track active PIDs independently
- Kill switch edge cases: PID no longer exists (session already ended — handle gracefully), multiple agents running simultaneously (kill all), child processes spawned by agent (use process group kill: `kill(-pgid, SIGKILL)`)
- Alert configuration screen in Settings: toggle alerts on/off, set path threshold ("alert if agent accesses paths outside [configurable directory]"), set blocked action threshold (default: 10 blocked actions in a session triggers alert)
- Alert delivery: macOS Notification Center via Tauri notification API, Linux libnotify fallback
- Alert log: store all triggered alerts in SQLite with timestamp, session_id, alert_type, details
- Test: run a session that deliberately exceeds the threshold. Verify alert fires and kill switch terminates it cleanly.

**Output checkpoint:** Active agent session running. Close the app window. Use system tray to kill the session. Verify it terminates within 500ms and the termination is logged.

---

### WEEK 8 — Repository policy workflows (v0.2.0)

**Goal:** A team of 10 developers can all use the same Rampart baseline profiles, managed via a file in their shared git repository without requiring the hosted product.

**Tasks:**
- Policy serialization: serialize all project profiles for a given directory to `.rampart/policy.json` in that directory. Format:
  ```json
  {
    "version": "1",
    "updated_at": "2026-04-08T12:00:00Z",
    "profiles": {
      "/absolute/path/to/project": {
        "template": "nextjs",
        "custom_rules": []
      }
    }
  }
  ```
- Policy import: on app startup, check if `.rampart/policy.json` exists in any currently open project directories. If found, offer to import. If policy.json was updated since last import, notify user.
- "Export team policy" button in project settings panel
- "Import from repository" button — also supports URL (e.g., raw GitHub URL) for teams who don't want to commit the file
- Conflict resolution: if local profile differs from policy.json, show diff and ask which to use
- Ship v0.2.0: update README, update CHANGELOG, and announce the new repository-backed workflow to users

**Output checkpoint:** Add `.rampart/policy.json` to a test git repo. Pull the repo on a second machine (or second user account). Open Rampart — it detects and imports the policy automatically.

---

### WEEK 9 — Lightweight web dashboard (team tier only)

**Goal:** An engineering manager can see all their team's sessions in one place without opening their own machine.

**Tasks:**
- Deploy minimal Fly.io instance: `fly launch` with a Go or Node.js API server + Postgres
- Auth: team tier Stripe webhook → create team record in Postgres → send magic link to team owner email
- API endpoints: `POST /sessions/upload` (desktop app pushes session summaries for team tier users), `GET /teams/:id/sessions` (dashboard reads), `POST /teams/:id/members/invite`
- Web dashboard frontend (separate from the desktop app — plain React, deployed to Fly.io): table of recent sessions across all team members, filter by date/member/agent, aggregate stats card (total sessions, total blocked actions, most common blocked paths)
- Desktop app: add "Sync to team dashboard" toggle in Settings. If enabled and user is on team tier, POST session summary to API on session end. No prompts, no code, no file contents — metadata only.
- EU region option: add `FLY_REGION` env var toggle, deploy second instance to Frankfurt on demand

**Output checkpoint:** Two machines both syncing sessions to the dashboard. Engineering manager sees both machines' sessions in the web UI.

---

### WEEK 10 — Linux headless CI mode

**Goal:** Rampart can be used in automated pipelines with no GUI.

**Tasks:**
- CLI binary: Tauri sidecar or standalone Rust binary: `rampart run --profile [template] --project-dir [path] --headless -- [agent_command]`
- CLI output: JSON session summary to stdout on completion: `{ "blocked_actions": 3, "files_read": [...], "exit_code": 0 }`
- Exit code contract: 0 = session completed within policy, 1 = session exceeded max-violations threshold, 2 = session terminated early (error)
- `--max-violations` flag: if blocked actions exceed this count, terminate the session and exit 1
- GitHub Action: `rampartHQ/sandbox-action@v1` with inputs: `agent`, `profile`, `project-dir`, `max-violations`. Outputs: `session-summary` JSON. Publishes session-summary as a workflow artifact.
- Publish to GitHub Actions Marketplace
- Write CI docs: step-by-step guide with ready-to-paste YAML for GitHub Actions, GitLab CI, CircleCI

**Output checkpoint:** A GitHub Actions workflow runs `claude` through Rampart. The workflow fails (exit 1) if the agent exceeds 5 blocked actions. Session summary is available as a downloadable artifact.

---

### WEEKS 11–12 — Content engine + first enterprise outreach

**Goal:** Content in market, newsletter coverage secured, first enterprise pilot started.

**Week 11 — Content:**
- Write and publish: "What files can Claude Code access on your machine?" — concrete post with screenshots, answers a question developers search for
- Write and publish: "The 60% problem: why most teams can't stop a misbehaving AI agent" — use McKinsey/Kiteworks stats, pitch to tl;dr sec and TLDR DevOps
- Write and publish: "What I found when I ran Claude Code without a sandbox for a month" — personal experience, real blocked events from your own sessions. Personal + technical = high engagement.
- Set up basic SEO: meta tags, sitemap, submit to Google Search Console

**Week 12 — Enterprise outreach:**
- Identify 20 companies from GitHub stars list + Discord members: 20–200 engineers, active Claude Code/Cursor usage visible from their public repos
- LinkedIn DM to engineering managers: personalize using their GitHub activity. Not a cold pitch — frame as seeking feedback: "I saw your team is using Claude Code — we're building Rampart, a sandbox for agent access. Would you try it for 30 days free in exchange for a 30-minute feedback call?"
- Offer: 30-day free team trial, white-glove onboarding (you personally set up their policy.json), dedicated Slack channel
- Target: 3–5 enterprise pilots running by end of week 12
- Draft MSA + DPA template. Use a lawyer for 1 hour of review. This is required before any enterprise signs.

**Output checkpoint:** 3 published articles, at least one newsletter coverage secured, 3+ enterprise pilots started, first enterprise contract proposal sent.

---

### WEEKS 13–16 — Enterprise features + first contract

**Goal:** First enterprise contract signed at $500–$2,000/month. SOC2 audit engagement started.

**Week 13:**
- Vanta or Drata: sign up, begin evidence collection. This starts the SOC2 Type II clock (6-month minimum).
- SSO: implement Google Workspace OAuth 2.0 for team dashboard login. Okta SAML as second priority.
- Data residency: deploy EU Fly.io instance, add region picker on team signup

**Week 14:**
- Advanced audit log features for enterprise pilots: custom retention periods, SIEM export (JSON Lines format for Splunk/Datadog ingestion), webhook on policy violation
- Enterprise onboarding flow: guided setup for team admins, policy.json generator wizard, bulk member invite via CSV

**Week 15:**
- Security FAQ page on website: "What data does Rampart collect?", "Is the audit log encrypted at rest?", "Can Rampart read my code?", "What happens if greywall crashes?". Enterprise procurement reads this.
- Privacy policy + DPA finalized (these need to be live before enterprise signs)

**Week 16:**
- Follow up on all enterprise pilots: call each pilot team, collect feedback, identify blockers to converting to paid
- First enterprise contract target: $500/month for 20 seats (=$25/seat — higher than team tier due to support + compliance)
- If no enterprise contract by end of week 16: do a postmortem. Was the outreach targeting the right person? Was the pilot too long? Was pricing wrong? Adjust and re-run.

**Output checkpoint:** SOC2 readiness platform active, SSO live, at least one enterprise contract signed or in final negotiation.

---

### WEEKS 17–20 — Scale and decision point

**Goal:** $3,000–$10,000 MRR, clear path to next phase, decision on funding vs. bootstrapping made.

**Week 17–18:**
- Measure everything: installs, active users (DAU/MAU), team seats, MRR, churn rate, top support issues, feature usage (which profiles are used most, what searches are run in audit log). Build a simple internal dashboard in Metabase or Grafana.
- Kill underused features: if less than 10% of users use a feature, deprecate or simplify it. Focus builds moat.
- Optimize the most-used paths: the profile picker and session start flow get used every single session — make them fast and frictionless.

**Week 19:**
- Non-Windows expansion decision: only proceed if 20+ paying customers have explicitly requested deeper macOS/Linux coverage and you have evidence it is blocking conversions. If yes, prioritize the next platform/runtime investment. If no, keep tightening the Windows-first loop and publish the platform roadmap transparently.
- Funding decision: with $3,000+ MRR, an enterprise contract, and the $392M RSAC 2026 agentic security funding signal, a $500K–$1M pre-seed from angels is realistic. Evaluate: do you want to raise? Does it accelerate the roadmap meaningfully? Could you grow to $10K MRR without it?

**Week 20:**
- Write a public "Month 5 update" post: what shipped, what the numbers look like, what comes next. This is marketing. Transparency is a distribution channel for devtools.
- Set goals for months 6–12: target MRR, feature milestones, platform expansions, team (solo or first hire)
- Update this document. Archive week 1–20 as history. Write week 21–40 goals.

**Output checkpoint:** $3,000+ MRR, funding or growth decision documented, roadmap for months 6–12 written, this document updated with actuals vs. targets for every week.

---

## 11. CODEX AGENT INSTRUCTIONS

When a Codex agent session works on Rampart, it must:

1. **Read this file first.** If this file has been updated since your last session, re-read the changed sections before writing any code.
2. **Identify which week's tasks you are working on.** Only implement tasks listed for the current week. Do not look ahead.
3. **Check architecture decisions** in Section 3 before making any structural change. If a decision in Section 3 needs to change, stop and update this file first.
4. **Follow the output checkpoint** at the end of each week before marking the week complete. If the checkpoint cannot be demonstrated, the week is not done.
5. **Update `CHANGELOG.md`** for every user-visible change. Format: `## [version] — date\n### Added\n### Fixed\n### Changed`.
6. **Never silently swallow errors.** Every error path must give the user: what happened, why, what to do next.
7. **Do not build UI for features in future weeks.** Placeholder buttons are fine; non-functional UI creates false impressions during testing.
8. **Write tests for the Rust backend.** Every function in `src-tauri/src/` that takes an argument and returns a Result must have at least one unit test covering the error case.
9. **Name things for their purpose.** `launch_sandboxed_session` is good. `run` is not. `build_greywall_command` is good. `helper` is not.
10. **Before ending a session,** summarize: what was built, what was tested, what is known to not work, and what the next session should start with.

---

## 12. QUICK REFERENCE — KEY STATS FOR PITCHING AND WRITING

Use these numbers. They are sourced and current as of April 2026.

- 60% of organizations cannot terminate a misbehaving AI agent (Kiteworks 2026)
- 88% of organizations had AI agent security incidents in the last year (2026 survey, 900+ executives)
- 82% of MCP servers had path traversal vulnerabilities in 2025 testing (AgentNode research)
- 80.9% of technical teams are in active deployment (not just testing) of AI agents (2026)
- $392M in agentic AI security funding in the 2 weeks around RSAC 2026
- $3.6B raised by the top 10 agentic AI security startups total
- 21.7% of open-source model package suggestions are hallucinated packages (UTSA/Virginia Tech study)
- AI-assisted commits leak secrets at 2x the baseline rate (GitGuardian State of Secrets Sprawl 2026)
- 24,008 unique secrets found in MCP configuration files on public GitHub (GitGuardian 2026)
- 40% of enterprise applications will embed task-specific AI agents by end of 2026 (Gartner)
- $172B projected AI cybersecurity market by 2029 (Gartner, 73.9% CAGR)

---

*Last updated: April 8, 2026. Update this document at the start of each new phase.*
