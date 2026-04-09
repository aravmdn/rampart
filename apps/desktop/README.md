# Desktop shell

Rampart desktop now runs as a Tauri app with a React + Vite frontend shell.

The intended product shape is a local launch-and-control console for agent sessions:

1. choose a project
2. choose an installed agent
3. choose or accept a profile
4. launch the sandboxed session
5. inspect live blocked and allowed activity
6. stop the session and review local history

The desktop shell should not drift into a generic security dashboard, and it does not need to replace every existing agent chat interface in the first product version.

## Commands

- `pnpm dev`
- `pnpm build`
- `pnpm build:web`
- `pnpm typecheck`
- `pnpm test`

## Structure

- `src/` holds the React desktop shell
- `src-tauri/` holds the native Tauri host crate and config

