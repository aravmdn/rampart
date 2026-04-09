# Rampart business model and scalability

## Purpose

This document defines what Rampart sells, what stays open and local, and how the product can scale without undermining the open-source or local-first thesis.

## Core commercial model

Rampart is not primarily selling access to a sandbox binary.

Rampart sells a control plane around agent execution:

- local enforcement and visibility for individual developers
- shared policy distribution and review for teams
- aggregated audit visibility across many developer machines
- alerts, response workflows, and organization-level controls
- support, compliance, and procurement readiness for larger customers

The open-source product drives adoption and trust. The paid product monetizes coordination, visibility, and operational convenience that appear once multiple users or teams are involved.

## Product layers

### 1. Open local product

This is the adoption layer and should remain valuable on its own:

- local desktop app
- local daemon and policy engine
- local profiles and presets
- local audit history
- local violation explanations
- optional repository-backed policy files such as `.rampart/policy.json`
- no required account
- no required cloud dependency

Principle:

- an individual developer should be able to get meaningful protection and visibility without paying or sending data to Rampart infrastructure

### 2. Hosted team control plane

This is the primary paid layer:

- aggregated session and audit views across many users
- team membership, access control, and admin workflows
- hosted policy distribution and versioning
- remote alerts, notifications, and response workflows
- manager-facing visibility into blocked actions, trends, and exceptions
- hosted retention and export controls

Principle:

- the paid plan should save time, reduce coordination overhead, and make policy and audit workflows operationally usable for teams

### 3. Enterprise controls

This is the procurement and risk-management layer:

- SSO
- data residency options
- SIEM export and webhook integrations
- advanced retention controls
- support and SLA terms
- contract, security review, and procurement readiness

Principle:

- enterprise pricing is justified by administrative requirements, support obligations, and compliance workflows rather than by withholding core local protection

## What Rampart is actually selling

Rampart is selling three things:

1. Trust
- deterministic control over what agents can touch

2. Usability
- policy and audit workflows that normal developers and engineering managers can actually operate

3. Coordination
- a shared control plane for teams using many agents across many machines

The sandbox itself is necessary but not sufficient. The commercial value comes from making enforcement understandable, governable, and repeatable at team scale.

## Why open source still works here

Open source supports the product instead of undermining it:

- security tools gain trust faster when users can inspect the code and data model
- local-first enforcement is more credible when users can run it without vendor lock-in
- the free/open product creates a wide top-of-funnel among individual developers
- teams pay when they need coordination, hosted services, operational convenience, and procurement support

Apache 2.0 means others can technically reuse the code. That is acceptable if Rampart's advantage comes from:

- product quality
- hosted experience
- fast iteration
- support
- buyer trust
- integrations and enterprise readiness

## Packaging principles

### Free

Include:

- full local single-user workflow
- local audit history
- local policy editing
- repository-backed policy import/export
- honest platform diagnostics

Do not include:

- required cloud sync
- hosted team dashboards
- centralized org administration

### Team

Include:

- hosted team dashboard
- multi-user audit aggregation
- hosted policy coordination and approval flows
- remote alerting and response workflows
- shared history and retention beyond local-only defaults

Do not rely on:

- charging for features that work fully offline through git or local files alone

### Enterprise

Include:

- SSO
- data residency
- compliance exports
- advanced support
- contractual terms

## Scalability model

### Technical scalability

Rampart scales technically through separation of concerns:

- desktop app for UX
- daemon for orchestration and persistence
- policy core for stable product semantics
- engine adapters for runtime-specific enforcement
- optional hosted control plane for team and enterprise workflows

This supports growth from:

- one developer on one machine
- to a small engineering team
- to a larger organization with central policy and audit requirements

### Economic scalability

Rampart scales economically through a layered funnel:

1. Free local adoption
- low-friction install, no sales, no cloud requirement

2. Team conversion
- managers pay once they need visibility across multiple developers and shared policy operations

3. Enterprise expansion
- larger customers pay for procurement, compliance, and support requirements

This keeps the free tier cheap to serve because most single-user activity stays local, while paid tiers justify hosted infrastructure through clear team-level value.

## What not to do

- do not charge for basic local protection
- do not make git-backed policy files the main paid differentiator
- do not force cloud dependency into the individual workflow
- do not market unsupported enforcement as enterprise-grade control
- do not drift into generic observability without a direct agent-control use case

## Current product decision

The current product direction is:

- Rampart remains open source and local-first
- the free tier includes meaningful single-user protection and repository-backed policy workflows
- the paid product centers on a hosted team control plane and enterprise operating features
- enterprise value comes from administration, visibility, compliance, and support, not from hiding the local sandbox behind a paywall
