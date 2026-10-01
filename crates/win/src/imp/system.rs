//! Memory size (to recommend a model that fits) and tying the model process to the app.

use std::sync::OnceLock;

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject,
};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

use crate::{Error, Memory};

pub fn memory() -> Option<Memory> {
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: `status` is initialised with its size.
    unsafe { GlobalMemoryStatusEx(&mut status) }.ok()?;
    Some(Memory {
        total_bytes: status.ullTotalPhys,
        available_bytes: status.ullAvailPhys,
    })
}

struct Job(HANDLE);
// SAFETY: a job handle can be used from any thread.
unsafe impl Send for Job {}
// SAFETY: as above; the handle is only passed to thread-safe Win32 calls.
unsafe impl Sync for Job {}

static JOB: OnceLock<Option<Job>> = OnceLock::new();

/// Puts a process in a job that Windows closes when the app exits (even if it crashes),
/// which kills the process: the model never outlives the app and holds RAM.
pub fn kill_with_app(pid: u32) -> Result<(), Error> {
    let job = JOB.get_or_init(|| {
        // SAFETY: creates an anonymous job object that lives for the whole process.
        unsafe {
            let handle = CreateJobObjectW(None, None).ok()?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const core::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
            .ok()?;
            Some(Job(handle))
        }
    });
    let Some(job) = job else {
        return Err(Error::Os("couldn't create a job object".into()));
    };
    // SAFETY: opens the child by pid only to add it to the job, then closes the handle.
    unsafe {
        let process = OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid)
            .map_err(|e| Error::Os(e.message()))?;
        let result = AssignProcessToJobObject(job.0, process).map_err(|e| Error::Os(e.message()));
        let _ = CloseHandle(process);
        result
    }
}
