//! Rampart headless CLI — `rampart run`
//!
//! Launches an AI agent under Rampart enforcement without the GUI. All Windows
//! enforcement primitives (Job Objects, WFP, Low Integrity, ETW) apply exactly
//! as in the desktop app because they run through the same `RampartService`.
//!
//! Usage:
//!   rampart run --agent <id> --profile <id> --project <path> [--wsl2]
//!
//! Preflight results are printed to stderr. Audit events are printed to stdout
//! as JSONL (one JSON object per line). The process exits once the agent exits
//! or Ctrl+C is received.

#[cfg(target_os = "windows")]
use engine_windows::WindowsEnforcer;
#[cfg(not(target_os = "windows"))]
use engine_greywall::GreywallAdapter;

use rampartd::{
    DaemonApi, LaunchSessionRequest, LocalDataStore, RampartService, SessionEventRecord,
    agent_tool_from_id, profiles_for_agent, run_preflight,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg(target_os = "windows")]
type ActiveEngine = WindowsEnforcer;
#[cfg(not(target_os = "windows"))]
type ActiveEngine = GreywallAdapter;

fn usage() -> ! {
    eprintln!("Usage: rampart run --agent <id> --profile <id> --project <path> [--wsl2]");
    eprintln!();
    eprintln!("  --agent   Agent ID (claude-code, codex, aider, goose, opencode, gemini, cursor, copilot)");
    eprintln!("  --profile Profile ID from the preset list (use --list-profiles to see options)");
    eprintln!("  --project Absolute path to the project directory to protect");
    eprintln!("  --wsl2    Run agent inside WSL2 (stronger isolation; requires WSL2 installed)");
    eprintln!();
    eprintln!("Other subcommands:");
    eprintln!("  rampart list-profiles --agent <id> --project <path>");
    std::process::exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // --help / -h: print usage and exit 0
    if args.iter().any(|a| a == "--help" || a == "-h") {
        eprintln!("Usage: rampart run --agent <id> --profile <id> --project <path> [--wsl2]");
        eprintln!();
        eprintln!("  --agent   Agent ID (claude-code, codex, aider, goose, opencode, gemini, cursor, copilot)");
        eprintln!("  --profile Profile ID from the preset list (use --list-profiles to see options)");
        eprintln!("  --project Absolute path to the project directory to protect");
        eprintln!("  --wsl2    Run agent inside WSL2 (stronger isolation; requires WSL2 installed)");
        eprintln!();
        eprintln!("Other subcommands:");
        eprintln!("  rampart list-profiles --agent <id> --project <path>");
        return;
    }

    if args.len() < 2 {
        usage();
    }

    match args[1].as_str() {
        "run" => cmd_run(&args[2..]),
        "list-profiles" => cmd_list_profiles(&args[2..]),
        _ => usage(),
    }
}

fn parse_flag<'a>(args: &'a [String], flag: &str) -> Option<&'a str> {
    for i in 0..args.len() {
        if args[i] == flag {
            return args.get(i + 1).map(|s| s.as_str());
        }
    }
    None
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|a| a == flag)
}

fn cmd_list_profiles(args: &[String]) {
    let agent_id = parse_flag(args, "--agent");
    let project = parse_flag(args, "--project").unwrap_or(".");
    let profiles = profiles_for_agent(agent_id, project);
    for p in &profiles {
        eprintln!("  {:30}  {}", p.id, p.name);
    }
}

fn build_service(data_dir: &str) -> RampartService<ActiveEngine> {
    #[cfg(target_os = "windows")]
    let engine = WindowsEnforcer::new();
    #[cfg(not(target_os = "windows"))]
    let engine = GreywallAdapter::discover().unwrap_or_else(|e| {
        eprintln!("rampart: engine init failed: {e}");
        std::process::exit(1);
    });

    let store = LocalDataStore::open(data_dir).unwrap_or_else(|e| {
        eprintln!("rampart: data store init failed: {e}");
        std::process::exit(1);
    });

    RampartService::new(engine, store).unwrap_or_else(|e| {
        eprintln!("rampart: service init failed: {e}");
        std::process::exit(1);
    })
}

fn cmd_run(args: &[String]) {
    let agent_id = parse_flag(args, "--agent").unwrap_or_else(|| {
        eprintln!("rampart: --agent is required");
        usage();
    });
    let profile_id = parse_flag(args, "--profile").unwrap_or_else(|| {
        eprintln!("rampart: --profile is required");
        usage();
    });
    let project = parse_flag(args, "--project").unwrap_or_else(|| {
        eprintln!("rampart: --project is required");
        usage();
    });
    let use_wsl2 = has_flag(args, "--wsl2");

    // Derive data dir from project path so each project gets its own store.
    let data_dir = format!("{project}/.rampart");
    let mut service = build_service(&data_dir);

    // Preflight
    let agent_tool = agent_tool_from_id(agent_id);
    let profiles = profiles_for_agent(Some(agent_id), project);
    let profile = profiles.iter().find(|p| p.id == profile_id).unwrap_or_else(|| {
        eprintln!("rampart: unknown profile '{profile_id}'. Run `rampart list-profiles --agent {agent_id} --project {project}` to see available profiles.");
        std::process::exit(1);
    });

    let capabilities = service.detect_capabilities().unwrap_or_else(|e| {
        eprintln!("rampart: capability detection failed: {e}");
        std::process::exit(1);
    });

    let preflight = run_preflight(project, &agent_tool, profile, &capabilities, None);

    eprintln!("=== Rampart preflight ===");
    for diag in &preflight.diagnostics {
        let icon = match diag.severity {
            rampartd::PreflightSeverity::Pass => "✓",
            rampartd::PreflightSeverity::Warning => "⚠",
            rampartd::PreflightSeverity::Fail => "✗",
        };
        eprintln!("  {icon} {}: {}", diag.label, diag.detail);
    }
    eprintln!("=========================");

    if !preflight.ready {
        eprintln!("rampart: preflight failed — session not started");
        std::process::exit(1);
    }

    // Launch
    let isolation_mode = if use_wsl2 {
        rampartd::IsolationMode::Wsl2
    } else {
        rampartd::IsolationMode::WindowsNative
    };

    let session = service
        .launch_session(LaunchSessionRequest {
            project_dir: project.to_string(),
            agent_id: agent_id.to_string(),
            profile_id: profile_id.to_string(),
            isolation_mode,
        })
        .unwrap_or_else(|e| {
            eprintln!("rampart: session launch failed: {e}");
            std::process::exit(1);
        });

    let session_id = session.id.clone();
    eprintln!("rampart: session {session_id} started (agent={agent_id}, profile={profile_id})");

    // Flush launch event immediately
    flush_new_events(&service, &session_id, &mut 0);

    // Ctrl+C handler
    let interrupted = Arc::new(AtomicBool::new(false));
    {
        let flag = interrupted.clone();
        ctrlc::set_handler(move || {
            flag.store(true, Ordering::SeqCst);
        })
        .unwrap_or_else(|e| eprintln!("rampart: could not set Ctrl+C handler: {e}"));
    }

    // Poll loop: flush new events every second; exit when agent exits or Ctrl+C.
    let mut last_flushed: usize = 0;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));

        flush_new_events(&service, &session_id, &mut last_flushed);

        if interrupted.load(Ordering::SeqCst) {
            eprintln!("rampart: interrupted — stopping session");
            break;
        }

        if !service.is_session_running(&session_id) {
            eprintln!("rampart: agent exited");
            break;
        }
    }

    // Stop and flush remaining events
    match service.stop_session(&session_id) {
        Ok(_) => {}
        Err(e) => eprintln!("rampart: stop_session error: {e}"),
    }

    // Flush any events that arrived during stop (including SessionEnded)
    flush_new_events(&service, &session_id, &mut last_flushed);
    eprintln!("rampart: session {session_id} ended");
}

fn flush_new_events(service: &RampartService<ActiveEngine>, session_id: &str, cursor: &mut usize) {
    let events = match service.stream_session_events(session_id) {
        Ok(e) => e,
        Err(_) => return,
    };
    for event in events.iter().skip(*cursor) {
        match event {
            SessionEventRecord::Audit(audit) => {
                if let Ok(json) = serde_json::to_string(audit) {
                    println!("{json}");
                }
            }
            SessionEventRecord::Violation(violation) => {
                if let Ok(json) = serde_json::to_string(violation) {
                    println!("{json}");
                }
            }
        }
    }
    *cursor = events.len();
}
