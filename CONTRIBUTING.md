# Contributing to Rampart

Thank you for your interest in contributing to Rampart! This guide will help you get set up and understand the project structure.

## Prerequisites

Install these before you begin:

- **Node.js** (LTS 20+) — for TypeScript and frontend tooling
- **pnpm** (9.x) — monorepo package manager
- **Rust via rustup** — with x86_64-pc-windows-msvc target on Windows
- **Visual Studio 2022 Build Tools** — with the "Desktop development with C++" workload (required for MSVC linker on Windows)
- **WSL2** (optional) — only required if you want to test the WSL2 isolation mode

## Development Setup

1. **Clone the repository:**
   ```sh
   git clone https://github.com/rampart-dev/rampart.git
   cd rampart
   ```

2. **Install dependencies:**
   ```sh
   pnpm install
   ```

3. **Build Rust workspace:**
   ```sh
   cargo build --workspace
   ```
   If this fails with `LNK1104` on Windows, you need Visual Studio 2022 Build Tools with the MSVC toolchain.

4. **Build shared-ui:**
   ```sh
   pnpm build:shared-ui
   ```

5. **Run the desktop app (dev mode):**
   ```sh
   pnpm dev:desktop
   ```
   This starts the Tauri dev server and watches for changes.

## Testing

Run the full test suite:

```sh
# Rust tests (requires MSVC linker; run from repo root or skip on systems without it)
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

# TypeScript / frontend tests
pnpm --filter @rampart/desktop exec vitest run
pnpm --filter @rampart/desktop exec tsc --noEmit
```

The desktop integration test (`App.test.tsx`) is the primary regression check.

## Running the CLI

The headless `rampart` binary works without the GUI:

```sh
# Build the CLI
cargo build -p rampart-cli

# Run a session
cargo run -p rampart-cli -- run --agent claude-code --profile claude-code.strict --project /path/to/project

# List available profiles for an agent
cargo run -p rampart-cli -- list-profiles --agent claude-code --project /path/to/project
```

The CLI outputs JSONL event stream to stdout and reads profiles from your local configuration.

## Code Style and Conventions

- **Default-deny posture:** any new policy logic should default to blocking access; explicit allow rules are required.
- **Enforcement outside model prompts:** security controls are enforced via OS/kernel primitives, not LLM instructions.
- **Never imply unsupported protections:** document what is actually enforced (see `threat-model.md`).
- **Public docs should not contain private working notes:** vault notes, internal roadmaps, and working decisions belong in the private vault, not in git.
- **Commit after each finished task:** do not leave uncommitted changes; do not commit node_modules, target/, vault/, or local state directories (see `.gitignore`).

See root `AGENTS.md` for full architecture guardrails.

## Reporting Issues and Getting Help

- **GitHub Issues:** Report bugs or suggest features on the [issues page](https://github.com/rampart-dev/rampart/issues).
- **Architecture and Design Questions:** See `docs/architecture.md`, `docs/threat-model.md`, and `docs/roadmap.md` for context.

## License

Contributions are licensed under Apache 2.0. See `LICENSE` for details.
