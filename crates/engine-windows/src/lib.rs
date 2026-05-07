use engine_greywall::{EnforcementEngine, GreywallAdapter, GreywallVersion, NormalizedEngineEvent, RawEngineEvent};
use policy_core::{
    CapabilitySupport, EngineCapabilitySnapshot, FilesystemCapabilitySnapshot,
    NetworkCapabilitySnapshot, ProcessCapabilitySnapshot,
};
use std::path::PathBuf;

pub const ENGINE_ID: &str = "windows-native";
pub const WSL2_ENGINE_ID: &str = "wsl2";

/// Windows-native enforcement engine.
///
/// Process containment is backed by Windows Job Objects (KILL_ON_JOB_CLOSE).
/// Filesystem write restriction uses Low Integrity tokens + project-root SACL.
/// Network enforcement uses WFP per-app-ID outbound filters.
pub struct WindowsEnforcer {
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
                // Low-integrity token + project-root SACL: agent cannot write to
                // Medium-or-higher integrity paths; project root is patched to Low
                // so the agent can write there.
                enforcement: CapabilitySupport::Limited,
                observation: CapabilitySupport::Unsupported,
                temporary_file_coverage: CapabilitySupport::Unsupported,
                atomic_rename_coverage: CapabilitySupport::Unsupported,
            },
            network: NetworkCapabilitySnapshot {
                // WFP per-app-ID outbound filter blocks network access.
                // WfpEventMonitor subscribes to classify-drop events so blocked
                // connections surface in the session console.
                enforcement: CapabilitySupport::Limited,
                observation: CapabilitySupport::Limited,
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

/// WSL2 enforcement engine (stronger isolation mode).
///
/// Surfaces `Supported` capabilities because the agent runs inside a Linux VM
/// where kernel-level enforcement (Landlock + seccomp) applies natively via the
/// existing greywall adapter. The host Win32 enforcement hooks are not used.
pub struct Wsl2Enforcer {
    greywall: GreywallAdapter,
}

impl Wsl2Enforcer {
    pub fn new() -> Self {
        Self {
            greywall: GreywallAdapter {
                binary_path: PathBuf::new(),
                version: GreywallVersion { major: 0, minor: 3, patch: 0 },
            },
        }
    }
}

impl Default for Wsl2Enforcer {
    fn default() -> Self {
        Self::new()
    }
}

impl EnforcementEngine for Wsl2Enforcer {
    fn engine_id(&self) -> &'static str {
        WSL2_ENGINE_ID
    }

    fn capability_snapshot(&self) -> EngineCapabilitySnapshot {
        EngineCapabilitySnapshot {
            engine_name: WSL2_ENGINE_ID.into(),
            engine_version: None,
            platform: "windows".into(),
            filesystem: FilesystemCapabilitySnapshot {
                enforcement: CapabilitySupport::Supported,
                observation: CapabilitySupport::Limited,
                temporary_file_coverage: CapabilitySupport::Supported,
                atomic_rename_coverage: CapabilitySupport::Supported,
            },
            network: NetworkCapabilitySnapshot {
                enforcement: CapabilitySupport::Supported,
                observation: CapabilitySupport::Limited,
                proxy_awareness: CapabilitySupport::Unsupported,
            },
            process: ProcessCapabilitySnapshot {
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

// ---------------------------------------------------------------------------
// Job Object — process tree containment
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
pub struct WindowsJob {
    handle: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(target_os = "windows")]
impl WindowsJob {
    pub fn assign(pid: u32) -> Result<Self, JobError> {
        use std::mem::{size_of, zeroed};
        use windows_sys::Win32::{
            Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
            System::{
                JobObjects::{
                    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
                    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
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

            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

            let ok = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &limits as *const _ as *const core::ffi::c_void,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
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
/// Called post-spawn. Best-effort; session continues on failure.
#[cfg(target_os = "windows")]
pub fn set_process_low_integrity(pid: u32) -> Result<(), LowIntegrityError> {
    use core::ffi::c_void;
    use core::mem::size_of;
    use windows_sys::Win32::{
        Foundation::CloseHandle,
        Security::{
            AllocateAndInitializeSid, FreeSid, GetLengthSid,
            SetTokenInformation, SID_AND_ATTRIBUTES, SID_IDENTIFIER_AUTHORITY,
            TOKEN_ADJUST_DEFAULT, TOKEN_MANDATORY_LABEL, TOKEN_QUERY, TokenIntegrityLevel,
        },
        System::Threading::{OpenProcess, OpenProcessToken, PROCESS_QUERY_INFORMATION},
    };

    const MANDATORY_LABEL_AUTHORITY: SID_IDENTIFIER_AUTHORITY =
        SID_IDENTIFIER_AUTHORITY { Value: [0, 0, 0, 0, 0, 16] };
    const MANDATORY_LOW_RID: u32 = 0x1000;
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
// Filesystem guard — project root SACL (Low mandatory label)
// ---------------------------------------------------------------------------

/// Set the mandatory integrity label on `path` to Low so that a Low-integrity
/// agent process can write to the project root.
///
/// By default directories inherit Medium integrity; a Low-integrity process
/// cannot write to them. Patching the SACL to Low allows the agent to write
/// while still blocking writes to all other Medium+ paths (user profile,
/// system dirs, etc.). Requires the caller to run at Medium+ integrity.
///
/// Best-effort: session continues if this fails.
#[cfg(target_os = "windows")]
pub fn patch_project_low_integrity_label(path: &std::path::Path) -> Result<(), SaclError> {
    use core::ffi::c_void;
    use core::mem::size_of;
    use windows_sys::Win32::Security::{
        ACL, AllocateAndInitializeSid, FreeSid, GetLengthSid, InitializeAcl,
        SID_IDENTIFIER_AUTHORITY,
    };
    use windows_sys::Win32::Security::Authorization::{
        SE_FILE_OBJECT, SetNamedSecurityInfoW,
    };
    // SYSTEM_MANDATORY_LABEL_NO_WRITE_UP and LABEL_SECURITY_INFORMATION are not
    // exported as named constants in windows-sys 0.52; use their raw values.
    const SYSTEM_MANDATORY_LABEL_NO_WRITE_UP: u32 = 0x00000001;
    const LABEL_SECURITY_INFORMATION: u32 = 0x00000010;

    // AddMandatoryAce is defined in Win32::Security but needs explicit import.
    use windows_sys::Win32::Security::AddMandatoryAce;

    let wide: Vec<u16> = path
        .to_string_lossy()
        .encode_utf16()
        .chain(std::iter::once(0u16))
        .collect();

    const MANDATORY_LABEL_AUTHORITY: SID_IDENTIFIER_AUTHORITY =
        SID_IDENTIFIER_AUTHORITY { Value: [0, 0, 0, 0, 0, 16] };
    const MANDATORY_LOW_RID: u32 = 0x1000;
    // ACL_REVISION = 2
    const ACL_REV: u32 = 2;

    unsafe {
        let mut low_sid: *mut c_void = core::ptr::null_mut();
        if AllocateAndInitializeSid(
            &MANDATORY_LABEL_AUTHORITY,
            1,
            MANDATORY_LOW_RID,
            0, 0, 0, 0, 0, 0, 0,
            &mut low_sid,
        ) == 0 {
            return Err(SaclError::AllocSidFailed);
        }

        let sid_len = GetLengthSid(low_sid) as usize;
        // Buffer: ACL header (8) + ACE header (4) + ACCESS_MASK (4) + SID bytes.
        let acl_size = size_of::<ACL>() + 8 + sid_len;
        let acl_words = acl_size.div_ceil(4);
        let mut acl_buf: Vec<u32> = vec![0u32; acl_words];
        let acl_ptr = acl_buf.as_mut_ptr() as *mut ACL;

        if InitializeAcl(acl_ptr, acl_size as u32, ACL_REV) == 0 {
            FreeSid(low_sid);
            return Err(SaclError::InitAclFailed);
        }

        // SYSTEM_MANDATORY_LABEL_NO_WRITE_UP: processes at Low IL can still
        // write because the directory's IL equals the process's IL.
        if AddMandatoryAce(acl_ptr, ACL_REV, 0, SYSTEM_MANDATORY_LABEL_NO_WRITE_UP, low_sid) == 0
        {
            FreeSid(low_sid);
            return Err(SaclError::AddAceFailed);
        }

        let result = SetNamedSecurityInfoW(
            wide.as_ptr(),
            SE_FILE_OBJECT,
            LABEL_SECURITY_INFORMATION,
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            acl_ptr,
        );

        FreeSid(low_sid);

        if result != 0 {
            Err(SaclError::SetSecurityInfoFailed(result))
        } else {
            Ok(())
        }
    }
}

#[derive(Debug)]
pub enum SaclError {
    AllocSidFailed,
    InitAclFailed,
    AddAceFailed,
    SetSecurityInfoFailed(u32),
}

impl std::fmt::Display for SaclError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaclError::AllocSidFailed => write!(f, "AllocateAndInitializeSid failed"),
            SaclError::InitAclFailed => write!(f, "InitializeAcl failed"),
            SaclError::AddAceFailed => write!(f, "AddMandatoryAce failed"),
            SaclError::SetSecurityInfoFailed(code) => {
                write!(f, "SetNamedSecurityInfoW failed with code {code:#010x}")
            }
        }
    }
}

impl std::error::Error for SaclError {}

// ---------------------------------------------------------------------------
// WFP network guard — per-app-ID outbound filter
// ---------------------------------------------------------------------------

/// Resolve a short command name (e.g. "claude") to a full Win32 path by
/// searching PATH. If the input already contains a path separator, returns it
/// unchanged. Tries `.exe`, `.cmd`, and `.bat` extensions in PATHEXT order so
/// that npm/pnpm-installed agent shims (`claude.cmd`) are located. Returns
/// `None` if the executable cannot be located under any extension.
#[cfg(target_os = "windows")]
fn resolve_app_path(command: &str) -> Option<String> {
    if command.contains('\\') || command.contains('/') {
        return Some(command.to_string());
    }
    use windows_sys::Win32::Storage::FileSystem::SearchPathW;

    let name_wide: Vec<u16> = command.encode_utf16().chain(std::iter::once(0u16)).collect();

    for ext in &[".exe\0", ".cmd\0", ".bat\0"] {
        let ext_wide: Vec<u16> = ext.encode_utf16().collect();
        let mut buf = vec![0u16; 1024];
        let mut file_part: *mut u16 = core::ptr::null_mut();

        let len = unsafe {
            SearchPathW(
                core::ptr::null(),
                name_wide.as_ptr(),
                ext_wide.as_ptr(),
                buf.len() as u32,
                buf.as_mut_ptr(),
                &mut file_part,
            )
        };
        if len != 0 {
            return Some(String::from_utf16_lossy(&buf[..len as usize]));
        }
    }
    None
}

/// Convert a Win32 path (e.g. `C:\path\to\app.exe`) to an NT device path
/// (e.g. `\Device\HarddiskVolume3\path\to\app.exe`) for use as a WFP app ID.
/// Returns `None` if the path does not start with a drive letter or
/// `QueryDosDeviceW` fails.
#[cfg(target_os = "windows")]
fn win32_to_nt_path(win32_path: &str) -> Option<String> {
    use windows_sys::Win32::Storage::FileSystem::QueryDosDeviceW;

    if win32_path.len() < 3 {
        return None;
    }
    let mut chars = win32_path.chars();
    let drive_letter = chars.next()?;
    if !drive_letter.is_ascii_alphabetic() || chars.next()? != ':' {
        return None;
    }

    let drive = &win32_path[..2]; // "C:"
    let drive_wide: Vec<u16> = drive.encode_utf16().chain(std::iter::once(0u16)).collect();
    let mut buf = vec![0u16; 512];

    unsafe {
        let len =
            QueryDosDeviceW(drive_wide.as_ptr(), buf.as_mut_ptr(), buf.len() as u32);
        if len == 0 {
            return None;
        }
        let device_end = buf.iter().position(|&c| c == 0).unwrap_or(len as usize);
        let device_path = String::from_utf16_lossy(&buf[..device_end]);
        let rest = &win32_path[2..]; // "\path\to\app.exe"
        Some(format!("{}{}", device_path, rest))
    }
}

/// WFP session that blocks outbound connections for a specific application.
///
/// Opens a dynamic WFP engine session (all objects cleaned up on close) and
/// adds per-app-ID BLOCK filters on the ALE connect layers for both IPv4 and
/// IPv6. The filters are automatically removed when this guard is dropped.
#[cfg(target_os = "windows")]
pub struct WfpNetworkGuard {
    engine: windows_sys::Win32::Foundation::HANDLE,
    /// NT device path of the monitored app (uppercased). Exposed so callers can
    /// start a `WfpEventMonitor` on the same path without re-resolving.
    pub nt_path: String,
}

#[cfg(target_os = "windows")]
impl WfpNetworkGuard {
    /// Open a WFP dynamic session and install outbound BLOCK filters for
    /// `app_path`. Returns `Err` if the engine cannot be opened, the path
    /// cannot be resolved, or the filter add fails. Non-fatal: the session
    /// continues without network enforcement.
    pub fn open(_pid: u32, app_path: &str) -> Result<Self, WfpError> {
        use core::mem::zeroed;
        use windows_sys::Win32::NetworkManagement::WindowsFilteringPlatform::{
            FwpmEngineClose0, FwpmEngineOpen0, FwpmFilterAdd0, FWPM_FILTER0,
            FWPM_FILTER_CONDITION0, FWPM_SESSION0, FWP_ACTION_BLOCK, FWP_BYTE_BLOB,
            FWP_BYTE_BLOB_TYPE, FWP_CONDITION_VALUE0, FWP_CONDITION_VALUE0_0,
            FWP_MATCH_EQUAL, FWPM_CONDITION_ALE_APP_ID, FWPM_LAYER_ALE_AUTH_CONNECT_V4,
            FWPM_LAYER_ALE_AUTH_CONNECT_V6,
        };

        let full_path = resolve_app_path(app_path).ok_or(WfpError::PathResolutionFailed)?;
        let nt_path = win32_to_nt_path(&full_path).ok_or(WfpError::PathResolutionFailed)?;
        // WFP app IDs are uppercased UTF-16LE with null terminator.
        let nt_upper = nt_path.to_uppercase();
        let mut nt_wide: Vec<u16> =
            nt_upper.encode_utf16().chain(std::iter::once(0u16)).collect();

        unsafe {
            let mut engine: windows_sys::Win32::Foundation::HANDLE = 0;
            let mut session: FWPM_SESSION0 = zeroed();
            // FWPM_SESSION_FLAG_DYNAMIC = 1: all created objects are deleted
            // automatically when the session (engine handle) is closed.
            session.flags = 1;
            let err =
                FwpmEngineOpen0(core::ptr::null(), 10, core::ptr::null(), &session, &mut engine);
            if err != 0 {
                return Err(WfpError::EngineOpenFailed(err));
            }

            let app_id = FWP_BYTE_BLOB {
                size: (nt_wide.len() * 2) as u32,
                data: nt_wide.as_mut_ptr() as *mut u8,
            };

            let condition = FWPM_FILTER_CONDITION0 {
                fieldKey: FWPM_CONDITION_ALE_APP_ID,
                matchType: FWP_MATCH_EQUAL,
                conditionValue: FWP_CONDITION_VALUE0 {
                    r#type: FWP_BYTE_BLOB_TYPE,
                    Anonymous: FWP_CONDITION_VALUE0_0 {
                        byteBlob: &app_id as *const FWP_BYTE_BLOB as *mut FWP_BYTE_BLOB,
                    },
                },
            };

            let mut filter: FWPM_FILTER0 = zeroed();
            // FwpmFilterAdd0 requires a non-null display name; provide one.
            let mut display_name: Vec<u16> =
                "Rampart outbound block\0".encode_utf16().collect();
            filter.displayData.name = display_name.as_mut_ptr();
            filter.numFilterConditions = 1;
            filter.filterCondition = &condition as *const FWPM_FILTER_CONDITION0
                as *mut FWPM_FILTER_CONDITION0;
            filter.action.r#type = FWP_ACTION_BLOCK;

            filter.layerKey = FWPM_LAYER_ALE_AUTH_CONNECT_V4;
            let mut id = 0u64;
            let err = FwpmFilterAdd0(engine, &filter, core::ptr::null_mut(), &mut id);
            if err != 0 {
                FwpmEngineClose0(engine);
                return Err(WfpError::FilterAddFailed(err));
            }

            filter.layerKey = FWPM_LAYER_ALE_AUTH_CONNECT_V6;
            let err = FwpmFilterAdd0(engine, &filter, core::ptr::null_mut(), &mut id);
            if err != 0 {
                // IPv4 filter is session-scoped; closing engine removes it.
                FwpmEngineClose0(engine);
                return Err(WfpError::FilterAddFailed(err));
            }

            Ok(Self { engine, nt_path: nt_upper })
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for WfpNetworkGuard {
    fn drop(&mut self) {
        unsafe {
            // Dynamic session: all filters auto-deleted on close.
            windows_sys::Win32::NetworkManagement::WindowsFilteringPlatform::FwpmEngineClose0(
                self.engine,
            );
        }
    }
}

#[cfg(target_os = "windows")]
unsafe impl Send for WfpNetworkGuard {}

#[derive(Debug)]
pub enum WfpError {
    EngineOpenFailed(u32),
    PathResolutionFailed,
    FilterAddFailed(u32),
}

impl std::fmt::Display for WfpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WfpError::EngineOpenFailed(code) => {
                write!(f, "FwpmEngineOpen0 failed with code {code:#010x}")
            }
            WfpError::PathResolutionFailed => {
                write!(f, "could not resolve app path to NT device path for WFP filter")
            }
            WfpError::FilterAddFailed(code) => {
                write!(f, "FwpmFilterAdd0 failed with code {code:#010x}")
            }
        }
    }
}

impl std::error::Error for WfpError {}

// ---------------------------------------------------------------------------
// ETW audit provider
// ---------------------------------------------------------------------------

/// Rampart audit ETW provider GUID.
/// {7E5A6B4C-F3D2-4A81-9B62-C1E0A4B8D7F6}
#[cfg(target_os = "windows")]
const RAMPART_PROVIDER_GUID: windows_sys::core::GUID = windows_sys::core::GUID {
    data1: 0x7E5A6B4C,
    data2: 0xF3D2,
    data3: 0x4A81,
    data4: [0x9B, 0x62, 0xC1, 0xE0, 0xA4, 0xB8, 0xD7, 0xF6],
};

/// ETW provider that emits audit events from the Rampart daemon.
///
/// Events can be captured with:
///   `logman start RampartTrace -p {7E5A6B4C-...} -o rampart.etl -ets`
///
/// Registration is best-effort; if `EventRegister` fails the struct is not
/// created and the daemon continues without ETW emission.
#[cfg(target_os = "windows")]
pub struct EtwAuditProvider {
    handle: u64, // REGHANDLE
}

#[cfg(target_os = "windows")]
impl EtwAuditProvider {
    pub fn register() -> Result<Self, EtwError> {
        use windows_sys::Win32::System::Diagnostics::Etw::EventRegister;
        unsafe {
            let mut handle: u64 = 0;
            let err = EventRegister(&RAMPART_PROVIDER_GUID, None, core::ptr::null(), &mut handle);
            if err != 0 {
                return Err(EtwError::RegisterFailed(err));
            }
            Ok(Self { handle })
        }
    }

    /// Emit a string audit event. Silently drops on failure.
    pub fn write_audit_event(
        &self,
        session_id: &str,
        kind: &str,
        outcome: &str,
        message: &str,
    ) {
        use windows_sys::Win32::System::Diagnostics::Etw::EventWriteString;
        let text = format!("[rampart] session={session_id} kind={kind} outcome={outcome} {message}");
        let mut wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0u16)).collect();
        unsafe {
            // level=0 (verbose), keyword=0 (all)
            EventWriteString(self.handle, 0, 0, wide.as_mut_ptr());
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for EtwAuditProvider {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::System::Diagnostics::Etw::EventUnregister(self.handle);
        }
    }
}

#[cfg(target_os = "windows")]
unsafe impl Send for EtwAuditProvider {}

#[derive(Debug)]
pub enum EtwError {
    RegisterFailed(u32),
}

impl std::fmt::Display for EtwError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EtwError::RegisterFailed(code) => {
                write!(f, "EventRegister failed with code {code:#010x}")
            }
        }
    }
}

impl std::error::Error for EtwError {}

// ---------------------------------------------------------------------------
// WFP event monitor — real-time blocked connection notifications
// ---------------------------------------------------------------------------

/// A blocked outbound connection captured from a WFP classify-drop event.
/// Not cfg-gated: the type is part of the public API on all platforms even
/// though it is only populated on Windows.
#[derive(Debug, Clone)]
pub struct BlockedNetworkEvent {
    pub remote_port: u16,
    pub occurred_at_ms: u64,
}

/// Callback context allocated on the heap for the lifetime of the subscription.
/// Holds the expected NT path (uppercased) and a shared event buffer.
#[cfg(target_os = "windows")]
type WfpMonitorCtx = (
    String,
    std::sync::Arc<std::sync::Mutex<Vec<BlockedNetworkEvent>>>,
);

/// WFP callback. Invoked on a BFE internal thread for every net event.
/// Filters to classify-drop events matching the expected app NT path and
/// pushes them into the shared buffer.
#[cfg(target_os = "windows")]
unsafe extern "system" fn wfp_event_callback(
    context: *mut core::ffi::c_void,
    event: *const windows_sys::Win32::NetworkManagement::WindowsFilteringPlatform::FWPM_NET_EVENT1,
) {
    use windows_sys::Win32::NetworkManagement::WindowsFilteringPlatform::FWPM_NET_EVENT_TYPE_CLASSIFY_DROP;

    if context.is_null() || event.is_null() {
        return;
    }
    let ctx = &*(context as *const WfpMonitorCtx);
    let expected = &ctx.0;
    let queue = &ctx.1;

    let ev = &*event;
    if ev.r#type != FWPM_NET_EVENT_TYPE_CLASSIFY_DROP {
        return;
    }

    // Decode the app ID blob (null-terminated UTF-16LE) and compare.
    let app_id = &ev.header.appId;
    if app_id.size == 0 || app_id.data.is_null() {
        return;
    }
    let app_bytes = core::slice::from_raw_parts(app_id.data, app_id.size as usize);
    if app_bytes.len() % 2 != 0 {
        return;
    }
    let words: Vec<u16> = app_bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let app_path_upper = String::from_utf16_lossy(&words)
        .trim_end_matches('\0')
        .to_uppercase();
    if app_path_upper != *expected {
        return;
    }

    // Convert FILETIME (100ns intervals since 1601-01-01) to Unix milliseconds.
    let ft = &ev.header.timeStamp;
    let filetime_100ns: u64 = ((ft.dwHighDateTime as u64) << 32) | ft.dwLowDateTime as u64;
    const EPOCH_DIFF_100NS: u64 = 116_444_736_000_000_000u64;
    let occurred_at_ms = filetime_100ns
        .checked_sub(EPOCH_DIFF_100NS)
        .map(|d| d / 10_000)
        .unwrap_or(0);

    if let Ok(mut guard) = queue.lock() {
        guard.push(BlockedNetworkEvent {
            remote_port: ev.header.remotePort,
            occurred_at_ms,
        });
    }
}

/// Subscribes to WFP net events and buffers classify-drop events for a specific
/// app (identified by its NT device path).
///
/// Uses a non-dynamic engine session so the subscription is not torn down by the
/// session flag. Must be dropped to unsubscribe — `Drop` calls
/// `FwpmNetEventUnsubscribe0` (which blocks until any in-flight callback returns)
/// then closes the engine handle and frees the callback context.
#[cfg(target_os = "windows")]
pub struct WfpEventMonitor {
    engine: windows_sys::Win32::Foundation::HANDLE,
    sub_handle: windows_sys::Win32::Foundation::HANDLE,
    events: std::sync::Arc<std::sync::Mutex<Vec<BlockedNetworkEvent>>>,
    ctx_raw: *mut WfpMonitorCtx,
}

#[cfg(target_os = "windows")]
impl WfpEventMonitor {
    /// Subscribe to WFP events for the app at `app_nt_path` (NT device path,
    /// uppercase). Returns `Err` on failure (e.g. insufficient privileges).
    pub fn start(app_nt_path: &str) -> Result<Self, WfpMonitorError> {
        use core::mem::zeroed;
        use windows_sys::Win32::NetworkManagement::WindowsFilteringPlatform::{
            FwpmEngineClose0, FwpmEngineOpen0, FwpmNetEventSubscribe0,
            FWPM_NET_EVENT_SUBSCRIPTION0, FWPM_SESSION0,
        };

        let events: std::sync::Arc<std::sync::Mutex<Vec<BlockedNetworkEvent>>> =
            std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let ctx: Box<WfpMonitorCtx> =
            Box::new((app_nt_path.to_string(), std::sync::Arc::clone(&events)));
        let ctx_raw = Box::into_raw(ctx);

        unsafe {
            let mut engine: windows_sys::Win32::Foundation::HANDLE = 0;
            let session: FWPM_SESSION0 = zeroed();
            // flags = 0: non-dynamic. Objects persist until explicitly deleted or
            // the engine handle is closed. The handle is our cleanup mechanism.
            let err = FwpmEngineOpen0(
                core::ptr::null(),
                10, // RPC_C_AUTHN_WINNT
                core::ptr::null(),
                &session,
                &mut engine,
            );
            if err != 0 {
                drop(Box::from_raw(ctx_raw));
                return Err(WfpMonitorError::EngineOpenFailed(err));
            }

            let mut sub: FWPM_NET_EVENT_SUBSCRIPTION0 = zeroed();
            // enumTemplate = null → receive all events; the callback filters by app ID.
            sub.enumTemplate = core::ptr::null_mut();

            let mut sub_handle: windows_sys::Win32::Foundation::HANDLE = 0;
            let err = FwpmNetEventSubscribe0(
                engine,
                &sub,
                Some(wfp_event_callback),
                ctx_raw as *const core::ffi::c_void,
                &mut sub_handle,
            );
            if err != 0 {
                FwpmEngineClose0(engine);
                drop(Box::from_raw(ctx_raw));
                return Err(WfpMonitorError::SubscribeFailed(err));
            }

            Ok(Self { engine, sub_handle, events, ctx_raw })
        }
    }

    /// Clone all pending events without consuming them. Used for live session
    /// polling so events remain in the buffer and appear on every poll.
    pub fn peek(&self) -> Vec<BlockedNetworkEvent> {
        self.events.lock().map(|g| g.clone()).unwrap_or_default()
    }

    /// Drain all pending events, emptying the buffer. Used at session stop to
    /// persist events into the permanent audit queue.
    pub fn drain(&self) -> Vec<BlockedNetworkEvent> {
        self.events
            .lock()
            .map(|mut g| std::mem::take(&mut *g))
            .unwrap_or_default()
    }
}

#[cfg(target_os = "windows")]
impl Drop for WfpEventMonitor {
    fn drop(&mut self) {
        unsafe {
            use windows_sys::Win32::NetworkManagement::WindowsFilteringPlatform::{
                FwpmEngineClose0, FwpmNetEventUnsubscribe0,
            };
            // Unsubscribe first — blocks until any in-progress callback returns.
            // After this returns, ctx_raw is guaranteed not to be accessed by WFP.
            FwpmNetEventUnsubscribe0(self.engine, self.sub_handle);
            FwpmEngineClose0(self.engine);
            drop(Box::from_raw(self.ctx_raw));
        }
    }
}

#[cfg(target_os = "windows")]
unsafe impl Send for WfpEventMonitor {}

#[derive(Debug)]
pub enum WfpMonitorError {
    EngineOpenFailed(u32),
    SubscribeFailed(u32),
}

impl std::fmt::Display for WfpMonitorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WfpMonitorError::EngineOpenFailed(code) => {
                write!(f, "FwpmEngineOpen0 for event monitor failed: {code:#010x}")
            }
            WfpMonitorError::SubscribeFailed(code) => {
                write!(f, "FwpmNetEventSubscribe0 failed: {code:#010x}")
            }
        }
    }
}

impl std::error::Error for WfpMonitorError {}
