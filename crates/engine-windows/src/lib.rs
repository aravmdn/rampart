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
                // Low-integrity token applied at spawn: agent cannot write to
                // Medium-or-higher integrity paths (user profile dirs, system
                // dirs). Reads are unrestricted. The project root requires an
                // explicit Low mandatory label to remain writable by the agent.
                enforcement: CapabilitySupport::Limited,
                observation: CapabilitySupport::Unsupported,
                temporary_file_coverage: CapabilitySupport::Unsupported,
                atomic_rename_coverage: CapabilitySupport::Unsupported,
            },
            network: NetworkCapabilitySnapshot {
                // WFP session opened; per-process filter implementation pending.
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

// ---------------------------------------------------------------------------
// Filesystem guard — Low Integrity token
// ---------------------------------------------------------------------------

/// Reduce the agent process to Low Integrity so the OS denies writes to all
/// Medium-or-higher integrity paths (user profile, system directories, etc.).
///
/// Called post-spawn. The process is already running; token integrity can be
/// *lowered* from outside with TOKEN_ADJUST_DEFAULT (requires the caller to
/// run at Medium or higher, which is the normal developer session).
///
/// **Limitation**: the project root directory typically has Medium mandatory
/// integrity. The agent cannot write there until the project root's mandatory
/// label is explicitly set to Low (SACL patching — Phase 3 follow-on). Until
/// then, coding agents that write files will fail with access denied on their
/// project directory. Tracked as a known gap in threat-model.md.
#[cfg(target_os = "windows")]
pub fn set_process_low_integrity(pid: u32) -> Result<(), LowIntegrityError> {
    use core::ffi::c_void;
    use core::mem::size_of;
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        Security::{
            AllocateAndInitializeSid, FreeSid, GetLengthSid, OpenProcessToken,
            SetTokenInformation, SID_AND_ATTRIBUTES, SID_IDENTIFIER_AUTHORITY,
            TOKEN_ADJUST_DEFAULT, TOKEN_MANDATORY_LABEL, TOKEN_QUERY, TokenIntegrityLevel,
        },
        System::Threading::{OpenProcess, PROCESS_QUERY_INFORMATION},
    };

    // S-1-16-4096  (Mandatory Label Authority = 16, Sub-authority = 0x1000)
    const MANDATORY_LABEL_AUTHORITY: SID_IDENTIFIER_AUTHORITY =
        SID_IDENTIFIER_AUTHORITY { Value: [0, 0, 0, 0, 0, 16] };
    const MANDATORY_LOW_RID: u32 = 0x1000;
    // SE_GROUP_INTEGRITY = 0x00000020
    const SE_GROUP_INTEGRITY: u32 = 0x00000020;

    unsafe {
        let proc = OpenProcess(PROCESS_QUERY_INFORMATION, 0, pid);
        if proc == 0 {
            return Err(LowIntegrityError::OpenProcessFailed(pid));
        }

        let mut token: windows_sys::Win32::Foundation::HANDLE = 0;
        if OpenProcessToken(proc, TOKEN_ADJUST_DEFAULT | TOKEN_QUERY, &mut token) == 0 {
            CloseHandle(proc);
            return Err(LowIntegrityError::OpenTokenFailed);
        }

        let mut low_sid: *mut c_void = core::ptr::null_mut();
        let ok = AllocateAndInitializeSid(
            &MANDATORY_LABEL_AUTHORITY,
            1,
            MANDATORY_LOW_RID,
            0, 0, 0, 0, 0, 0, 0,
            &mut low_sid,
        );
        if ok == 0 {
            CloseHandle(token);
            CloseHandle(proc);
            return Err(LowIntegrityError::AllocSidFailed);
        }

        let label = TOKEN_MANDATORY_LABEL {
            Label: SID_AND_ATTRIBUTES {
                Sid: low_sid,
                Attributes: SE_GROUP_INTEGRITY,
            },
        };
        let label_size =
            size_of::<TOKEN_MANDATORY_LABEL>() as u32 + GetLengthSid(low_sid);

        let ok = SetTokenInformation(
            token,
            TokenIntegrityLevel,
            &label as *const _ as *const c_void,
            label_size,
        );

        FreeSid(low_sid);
        CloseHandle(token);
        CloseHandle(proc);

        if ok == 0 {
            Err(LowIntegrityError::SetIntegrityFailed)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
pub enum LowIntegrityError {
    OpenProcessFailed(u32),
    OpenTokenFailed,
    AllocSidFailed,
    SetIntegrityFailed,
}

impl std::fmt::Display for LowIntegrityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LowIntegrityError::OpenProcessFailed(pid) => {
                write!(f, "OpenProcess failed for pid {pid}")
            }
            LowIntegrityError::OpenTokenFailed => write!(f, "OpenProcessToken failed"),
            LowIntegrityError::AllocSidFailed => write!(f, "AllocateAndInitializeSid failed"),
            LowIntegrityError::SetIntegrityFailed => {
                write!(f, "SetTokenInformation(TokenIntegrityLevel) failed")
            }
        }
    }
}

impl std::error::Error for LowIntegrityError {}

// ---------------------------------------------------------------------------
// WFP network guard — per-process outbound filter
// ---------------------------------------------------------------------------

/// WFP session that will block outbound connections for a specific process.
///
/// Currently opens the WFP engine and holds a transaction-ready handle.
/// Per-application-ID filter add is the Phase 3 follow-on: it requires
/// resolving the NT device path from the Win32 executable path, which needs
/// additional implementation and testing.
#[cfg(target_os = "windows")]
pub struct WfpNetworkGuard {
    engine: windows_sys::Win32::Foundation::HANDLE,
    filter_id: u64,
}

#[cfg(target_os = "windows")]
impl WfpNetworkGuard {
    /// Open a WFP engine session. Per-process filter add is not yet wired;
    /// returns `Err(WfpError::FilterNotImplemented)` until Phase 3 follow-on.
    pub fn open(_pid: u32, _app_path: &str) -> Result<Self, WfpError> {
        use windows_sys::Win32::NetworkManagement::WindowsFilteringPlatform::{
            FwpmEngineOpen0, FWPM_SESSION0,
        };
        use core::mem::zeroed;

        unsafe {
            let mut engine: windows_sys::Win32::Foundation::HANDLE = 0;
            let session: FWPM_SESSION0 = zeroed();
            // RPC_C_AUTHN_WINNT = 10
            let err = FwpmEngineOpen0(core::ptr::null(), 10, core::ptr::null(), &session, &mut engine);
            if err != 0 {
                return Err(WfpError::EngineOpenFailed(err));
            }
            // Per-application-ID filter add requires NT device path resolution.
            // Tracked for Phase 3 completion.
            Err(WfpError::FilterNotImplemented)
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for WfpNetworkGuard {
    fn drop(&mut self) {
        use windows_sys::Win32::NetworkManagement::WindowsFilteringPlatform::{
            FwpmEngineClose0, FwpmFilterDeleteById0,
        };
        unsafe {
            if self.filter_id != 0 {
                FwpmFilterDeleteById0(self.engine, self.filter_id);
            }
            FwpmEngineClose0(self.engine);
        }
    }
}

// HANDLE is a kernel object handle; safe to send across threads.
#[cfg(target_os = "windows")]
unsafe impl Send for WfpNetworkGuard {}

#[derive(Debug)]
pub enum WfpError {
    EngineOpenFailed(u32),
    FilterNotImplemented,
}

impl std::fmt::Display for WfpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WfpError::EngineOpenFailed(code) => write!(f, "FwpmEngineOpen0 failed with code {code:#010x}"),
            WfpError::FilterNotImplemented => {
                write!(f, "per-process WFP filter not yet implemented (Phase 3 follow-on)")
            }
        }
    }
}

impl std::error::Error for WfpError {}
