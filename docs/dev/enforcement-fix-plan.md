# Enforcement Fix Plan: WFP for .cmd Shim Agents

**Status**: Planned — not yet implemented  
**Created**: 2026-05-07

---

## Problem Summary

Three interconnected problems prevent WFP network enforcement from working for npm-installed agents (Claude Code, Codex, Aider):

1. **WFP filter targets wrong executable.** `resolve_app_path("claude")` returns `claude.cmd`'s NT path. WFP matches by process image path. The actual process making connections is `node.exe` — the filter never fires.

2. **Enforcement chain broken by wt.exe launch.** The `wt.exe` workaround (introduced to fix the blank terminal) makes `wt.exe` the direct child. Job Object and Low Integrity token target `wt.exe`, not the agent. `node.exe` runs unrestricted.

3. **Blank terminal when Tauri has a parent console.** Rust's `Command` always sets `STARTF_USESTDHANDLES` in `STARTUPINFO`, inheriting Tauri's console handles. When Tauri is launched from an admin terminal, `CREATE_NEW_CONSOLE` opens a new window but the child's stdio points at Tauri's terminal — the new window is blank. This was the root cause of the `wt.exe` workaround.

---

## Solution

Replace `wt.exe` delegation with raw `CreateProcessW` (no `STARTF_USESTDHANDLES`). Windows auto-attaches stdin/stdout/stderr to the new console when `STARTF_USESTDHANDLES` is absent and `CREATE_NEW_CONSOLE` is set. This restores the direct child PID, which restores the full enforcement chain.

Fix WFP target: when `resolve_app_path` returns a `.cmd`/`.bat`, read the shim file to identify the interpreter (`node`, `python`, etc.) and use that executable's PATH-resolved path as the WFP filter target instead.

**Terminal change**: `wt.exe` (Windows Terminal) → `conhost.exe` (legacy console host). Functionally identical for interactive CLI agents; less polished visually.

**WFP known limitation**: The filter targets `node.exe` by path. During a session ALL outbound from `node.exe` is blocked (not just this session's process). Acceptable for intentional sandboxing; document in GETTING_STARTED.

---

## Files to Change

| File | What changes |
|---|---|
| `crates/engine-windows/src/lib.rs` | Add `ConsolePid`, `spawn_in_new_console`, `resolve_interpreter_path`; fix `WfpNetworkGuard::open` |
| `crates/rampartd/src/lib.rs` | Add `console_children` field; update `spawn_session`, `stop_session`, `is_session_running` |

No Cargo.toml changes — `Win32_System_Threading` is already in engine-windows features.

---

## Implementation Steps

### 1. `ConsolePid` struct (engine-windows)

Wraps a raw Windows `HANDLE` + PID for processes spawned via `CreateProcessW`.

```rust
pub struct ConsolePid { pid: u32, handle: HANDLE }
impl ConsolePid {
    pub fn id(&self) -> u32 { self.pid }
    pub fn kill(&self) { unsafe { TerminateProcess(self.handle, 1); } }
    pub fn try_is_running(&self) -> bool {
        // WaitForSingleObject(handle, 0): returns WAIT_OBJECT_0(0) if exited, WAIT_TIMEOUT(258) if running
        unsafe { WaitForSingleObject(self.handle, 0) != 0 }
    }
}
impl Drop for ConsolePid { fn drop(&mut self) { unsafe { CloseHandle(self.handle); } } }
```

### 2. `spawn_in_new_console` (engine-windows)

Raw `CreateProcessW` without `STARTF_USESTDHANDLES`:

```rust
pub fn spawn_in_new_console(command_line: &str, working_dir: &str) -> Result<ConsolePid, io::Error> {
    let mut si: STARTUPINFOW = zeroed();
    si.cb = size_of::<STARTUPINFOW>() as u32;
    // dwFlags intentionally NOT set to STARTF_USESTDHANDLES
    // → Windows auto-attaches child stdio to the new console's buffers

    let mut pi: PROCESS_INFORMATION = zeroed();
    let mut cmd_wide: Vec<u16> = command_line.encode_utf16().chain(once(0)).collect();
    let cwd_wide: Vec<u16> = working_dir.encode_utf16().chain(once(0)).collect();

    let ok = CreateProcessW(
        null(), cmd_wide.as_mut_ptr(), null(), null(),
        0,                  // bInheritHandles = FALSE
        CREATE_NEW_CONSOLE,
        null(), cwd_wide.as_ptr(), &si, &mut pi,
    );
    if ok == 0 { return Err(io::Error::last_os_error()); }
    CloseHandle(pi.hThread);
    Ok(ConsolePid { pid: pi.dwProcessId, handle: pi.hProcess })
}
```

### 3. `resolve_interpreter_path` (engine-windows)

Parse the shim file and find the actual runtime executable:

```rust
fn resolve_interpreter_path(cmd_path: &str) -> Option<String> {
    let content = std::fs::read_to_string(cmd_path).ok()?;
    let lower = content.to_lowercase();
    let interpreter = if lower.contains("node") { "node" }
                      else if lower.contains("python") { "python" }
                      else if lower.contains("ruby") { "ruby" }
                      else { return None; };

    // Check same directory first (npm global installs co-locate node.exe)
    let co = Path::new(cmd_path).parent()?.join(format!("{interpreter}.exe"));
    if co.exists() { return Some(co.to_string_lossy().into_owned()); }

    resolve_app_path(interpreter)  // fall back to PATH
}
```

### 4. Fix `WfpNetworkGuard::open` (engine-windows)

After `resolve_app_path` returns a `.cmd`/`.bat`, redirect to interpreter:

```rust
let wfp_target = if full_path.to_ascii_lowercase().ends_with(".cmd")
                 || full_path.to_ascii_lowercase().ends_with(".bat") {
    resolve_interpreter_path(&full_path).unwrap_or_else(|| full_path.clone())
} else {
    full_path.clone()
};
let nt_path = win32_to_nt_path(&wfp_target).ok_or(WfpError::PathResolutionFailed)?;
// ... rest unchanged
```

### 5. `console_children` field (rampartd)

Add to `LocalProcessRunner`:

```rust
#[cfg(target_os = "windows")]
console_children: HashMap<String, engine_windows::ConsolePid>,
```

### 6. Update `spawn_session` (rampartd)

Replace `else if use_wt { Command::new("wt") ... }` branch: call `engine_windows::spawn_in_new_console` instead. All enforcement (Job Object, Low Integrity, WFP) is re-enabled — `if !use_wt` guards are removed.

Build the command line string:
- shim: `format!("cmd.exe /K {}", adapter.command)`
- direct: `adapter.command.clone()`

### 7. Update `stop_session` (rampartd)

```rust
fn stop_session(&mut self, session_id: &str) -> Result<(), ProcessRunnerError> {
    #[cfg(target_os = "windows")] drop(self.jobs.remove(session_id));
    #[cfg(target_os = "windows")] drop(self.wfp_guards.remove(session_id));
    #[cfg(target_os = "windows")] drop(self.wfp_monitors.remove(session_id));

    #[cfg(target_os = "windows")]
    if let Some(child) = self.console_children.remove(session_id) {
        child.kill();
        return Ok(());
    }

    let mut child = self.children.remove(session_id)
        .ok_or_else(|| ProcessRunnerError::UnknownSession(session_id.into()))?;
    child.kill().map_err(|source| ProcessRunnerError::Stop { session_id: session_id.into(), source })
}
```

### 8. Update `is_session_running` (rampartd)

```rust
pub fn is_session_running(&mut self, session_id: &str) -> bool {
    #[cfg(target_os = "windows")]
    if let Some(c) = self.console_children.get(session_id) { return c.try_is_running(); }
    match self.children.get_mut(session_id) {
        Some(c) => c.try_wait().map(|s| s.is_none()).unwrap_or(false),
        None => false,
    }
}
```

---

## Enforcement State After Fix

| Primitive | Before this plan | After |
|---|---|---|
| Terminal interactive | ✓ wt.exe | ✓ conhost.exe (raw CreateProcessW) |
| Job Object | ✗ targets wt.exe | ✓ cmd.exe + node.exe in job |
| Low Integrity token | ✗ skipped | ✓ cmd.exe → node.exe inherits |
| Filesystem (Low IL + SACL) | ✗ Low IL not applied | ✓ writes outside project root denied |
| WFP network block | ✗ targets claude.cmd | ✓ targets node.exe |
| WFP event monitor | ✗ wrong path | ✓ correct path |

---

## Verification Steps

1. Run Rampart as Administrator
2. Launch session: Claude Code agent, `claude-code.strict` profile, any project dir
3. Terminal: `conhost.exe` window opens, Claude Code TUI renders, keyboard works
4. In Claude: `curl https://ifconfig.me` → connection fails (WFP blocks node.exe outbound)
5. Violation View in Rampart: WFP network violation card appears
6. Stop session: terminal window closes
7. `pnpm --filter @rampart/desktop exec vitest run` → 33 tests pass
