# Claude Code Skills

Quick reference for available Claude Code skills. Invoke with `/skillname`.

## Workflow & Git

| Skill | Purpose |
|-------|---------|
| `/plan-work` | Plan work before coding: repo research, analyze options/risks, clarify requirements |
| `/commit-work` | Create high-quality git commits: review/stage changes, split logical chunks, follow conventions |
| `/create-pr` | Create a high-quality pull request: branch, focused changes, lint/build, conventional messaging |
| `/review` | Review a pull request |
| `/security-review` | Complete a security review of pending changes on current branch |
| `/bug-triage` | Reproduce, isolate, and fix a bug; summarize root cause and fix rationale |
| `/ci-fix` | Fix GitHub Actions CI failures: inspect runs/logs, identify root cause, propose fixes |
| `/branch-cleaner` | Identify and clean up stale git branches locally and on remotes safely |
| `/rebase-assistant` | Guide a safe git rebase onto target branch, including conflict resolution |

## Documentation

| Skill | Purpose |
|-------|---------|
| `/agents-md` | Create or update root and nested AGENTS.md files that document scoped conventions |
| `/docs-sync` | Keep documentation in sync with code changes across README, docs sites, API docs |
| `/release-notes` | Draft release notes and changelog entries from git history or merged PRs |
| `/sessions-to-blog` | Generate MDX blog posts from session logs in `sessions/articles` |

## Language & Framework Specific

| Skill | Purpose |
|-------|---------|
| `/claude-api` | Build, debug, optimize Claude API / Anthropic SDK apps; handle model migrations; prompt caching |
| `/dependency-upgrader` | Upgrade dependencies for Java/Kotlin (Gradle/Maven) and TypeScript/Node projects |
| `/ui-ux-pro-max` | Design, build, refine frontend UI/UX: layouts, components, visual polish |
| `/coding-guidelines-gen` | Generate nested AGENTS.md coding guidelines per module (monorepo-aware) |
| `/coding-guidelines-verify` | Verify changes follow nearest-scoped AGENTS.md rules |

## Infrastructure & Deployment

| Skill | Purpose |
|-------|---------|
| `/microsoft-foundry` | Deploy, evaluate, manage Foundry agents end-to-end: Docker build, ACR push, helm |
| `/vps-checkup` | SSH into Ubuntu VPS (Docker) for health/security/update report |
| `/create-cli` | Design CLI parameters and UX: arguments, flags, subcommands, help |
| `/simplify` | Review changed code for reuse, quality, efficiency; fix issues found |

## Azure Suite

| Skill | Purpose |
|-------|---------|
| `/azure-ai` | Azure AI: Search, Speech, OpenAI, Document Intelligence |
| `/azure-aigateway` | Configure Azure API Management as an AI Gateway |
| `/azure-cloud-migrate` | Assess and migrate cross-cloud workloads to Azure |
| `/azure-compliance` | Comprehensive Azure compliance and security auditing |
| `/azure-compute` | Azure VM and VMSS recommendations, pricing, autoscale |
| `/azure-cost-optimization` | Identify and quantify cost savings across Azure subscriptions |
| `/azure-deploy` | Execute Azure deployments for prepared applications |
| `/azure-diagnostics` | Debug Azure production issues using AppLens, Azure Monitor |
| `/azure-enterprise-infra-planner` | Architect and provision enterprise Azure infrastructure |
| `/azure-hosted-copilot-sdk` | Build and deploy GitHub Copilot SDK apps to Azure |
| `/azure-kusto` | Query and analyze data in Azure Data Explorer (Kusto/ADX) |
| `/azure-messaging` | Troubleshoot Azure Messaging SDKs for Event Hubs and Service Bus |
| `/azure-prepare` | Prepare Azure apps for deployment: infra Bicep/Terraform, azure.yaml, Dockerfiles |
| `/azure-quotas` | Check/manage Azure quotas and usage across providers |
| `/azure-rbac` | Find the right Azure RBAC role with least privilege |
| `/azure-resource-lookup` | List, find, show Azure resources |
| `/azure-resource-visualizer` | Generate detailed Mermaid architecture diagrams from Azure resource groups |
| `/azure-storage` | Azure Storage Services: Blob, File Shares, Queue, Table |
| `/azure-upgrade` | Assess and upgrade Azure workloads between plans, tiers, SKUs |
| `/azure-validate` | Pre-deployment validation for Azure readiness |

## Configuration & Setup

| Skill | Purpose |
|-------|---------|
| `/update-config` | Configure Claude Code harness via settings.json; manage hooks, permissions, env vars |
| `/keybindings-help` | Customize keyboard shortcuts, rebind keys, add chord bindings |
| `/loop` | Run a prompt or slash command on a recurring interval |
| `/schedule` | Create, update, list, run scheduled remote agents on cron schedule |
| `/init` | Initialize a new CLAUDE.md file with codebase documentation |
| `/fewer-permission-prompts` | Scan transcripts for common calls; add allowlist to reduce permission prompts |
| `/regex-builder` | Build, test, explain regular expressions against sample text |

## Utilities

| Skill | Purpose |
|-------|---------|
| `/entra-app-registration` | Guide Microsoft Entra ID app registration, OAuth 2.0, MSAL |
| `/appinsights-instrumentation` | Instrument webapps with Azure Application Insights |
| `/video-transcript-downloader` | Download videos, audio, subtitles, clean transcripts from YouTube |

---

**Usage:** `/skillname` or `Skill(skill: "skillname")`

**Examples:**
- `/plan-work` — before starting a feature
- `/commit-work` — after finishing a feature
- `/create-pr` — when ready to merge
- `/security-review` — before merging sensitive changes
- `/update-config` — to add permissions or env vars
- `/azure-deploy` — to deploy prepared apps
