use engine_greywall::{EnforcementEngine, GreywallAdapter, GreywallVersion, NormalizedEngineEvent, RawEngineEvent};
use policy_core::{
    CapabilitySupport, EngineCapabilitySnapshot, FilesystemCapabilitySnapshot,
    NetworkCapabilitySnapshot, ProcessCapabilitySnapshot,
};
use std::path::PathBuf;

pub const ENGINE_ID: &str = "windows-native";

/// Windows-native enforcement engine.
///
/// Process containment is backed by Windows Job Objects (KILL_ON_JOB_CLOSE).
/// Filesystem and network enforcement are not yet applied; those capabilities
/// are reported as Unsupported until ACLs and WFP are wired in Phase 3.
pub struct WindowsEnforcer {
    // Greywall adapter used solely for event normalization so that the audit
    // taxonomy remains consistent across engines.
    greywall: GreywallAdapter,
}

impl WindowsEnforcer {
    pub fn new() -> Self {
        Self {
            greywall: GreywallAdapter {
                binary_path: PathBuf::new(),
                version: GreywallVersion { major: 0, minor: 3, patch: 0 },
            },
        }
    }
}

impl Default for WindowsEnforcer {
    fn default() -> Self {
        Self::new()
    }
}

impl EnforcementEngine for WindowsEnforcer {
    fn engine_id(&self) -> &'static str {
        ENGINE_ID
    }

    fn capability_snapshot(&self) -> EngineCapabilitySnapshot {
        EngineCapabilitySnapshot {
            engine_name: ENGINE_ID.into(),
            engine_version: None,
            platform: "windows".into(),
            filesystem: FilesystemCapabilitySnapshot {
                // Directory ACLs not yet applied — Phase 3 follow-on.
                enforcement: CapabilitySupport::Unsupported,
                observation: CapabilitySupport::Unsupported,
                temporary_file_coverage: CapabilitySupport::Unsupported,
                atomic_rename_coverage: CapabilitySupport::Unsupported,
            },
            network: NetworkCapabilitySnapshot {
                // WFP rules not yet applied — Phase 3 follow-on.
                enforcement: CapabilitySupport::Unsupported,
                observation: CapabilitySupport::Unsupported,
                proxy_awareness: CapabilitySupport::Unsupported,
            },
            process: ProcessCapabilitySnapshot {
                // Job Objects: agent process tree is contained. OS kills all
                // children when the job handle is dropped at session end.
                enforcement: CapabilitySupport::Supported,
                observation: CapabilitySupport::Limited,
                termination: CapabilitySupport::Supported,
            },
        }
    }

    fn normalize_event(&self, event: RawEngineEvent) -> NormalizedEngineEvent {
        self.greywall.normalize_event(event)
    }
}

/// Handle to an OS-level Job Object for a single session.
///
/// The job has KILL_ON_JOB_CLOSE set; when this handle is dropped (session end
/// or panic), the OS terminates every process still in the job.
#[cfg(target_os = "windows")]
pub struct WindowsJob {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(target_os = "windows")]
impl WindowsJob {
    /// Create a new job object and assign the process identified by `pid` to it.
    ///
    /// Returns `Err` if any Win32 call fails. The caller should treat this as a
    /// non-fatal warning; the session still runs, just without job containment.
    pub fn assign(pid: u32) -> Result<Self, JobError> {
        use std::mem::{size_of, zeroed};
        use windows_sys::Win32::{
            Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
            System::{
                JobObjects::{
                    AssignProcessToJobObject, CreateJobObjectW, JobObjectBasicLimitInformation,
                    SetInformationJobObject, JOBOBJECT_BASIC_LIMIT_INFORMATION,
                    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                },
                Threading::{OpenProcess, PROCESS_ALL_ACCESS},
            },
        };

        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job == 0 || job == INVALID_HANDLE_VALUE {
                return Err(JobError::CreateFailed);
            }

            let mut limits: JOBOBJECT_BASIC_LIMIT_INFORMATION = zeroed();
            limits.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let ok = SetInformationJobObject(
                job,
                JobObjectBasicLimitInformation,
                &limits as *const _ as *const core::ffi::c_void,
                size_of::<JOBOBJECT_BASIC_LIMIT_INFORMATION>() as u32,
            );
            if ok == 0 {
                CloseHandle(job);
                return Err(JobError::ConfigureFailed);
            }

            let proc = OpenProcess(PROCESS_ALL_ACCESS, 0, pid);
            if proc == 0 {
                CloseHandle(job);
                return Err(JobError::OpenProcessFailed(pid));
            }

            let ok = AssignProcessToJobObject(job, proc);
            CloseHandle(proc);

            if ok == 0 {
                CloseHandle(job);
                return Err(JobError::AssignFailed);
            }

            Ok(Self { handle: job })
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for WindowsJob {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

// HANDLE is a kernel object handle; it is valid to send across threads.
#[cfg(target_os = "windows")]
unsafe impl Send for WindowsJob {}

#[derive(Debug)]
pub enum JobError {
    CreateFailed,
    ConfigureFailed,
    OpenProcessFailed(u32),
    AssignFailed,
}

impl std::fmt::Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobError::CreateFailed => write!(f, "CreateJobObjectW failed"),
            JobError::ConfigureFailed => write!(f, "SetInformationJobObject failed"),
            JobError::OpenProcessFailed(pid) => write!(f, "OpenProcess failed for pid {pid}"),
            JobError::AssignFailed => write!(f, "AssignProcessToJobObject failed"),
        }
    }
}

impl std::error::Error for JobError {}
