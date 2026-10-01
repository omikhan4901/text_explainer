//! The clipboard fallback and the double-copy trigger.
//!
//! Fallback: snapshot every clipboard format, send Ctrl+C, read the text once the
//! clipboard changes, and put the snapshot back exactly. The restore is marked so
//! Windows clipboard history and cloud clipboard don't record it.
//!
//! Double copy: pressing Ctrl+C twice within a moment (with the same text) explains the
//! copied text. This works in every app that can copy, with no simulated keys at all.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, EnumClipboardFormats,
    GetClipboardData, GetClipboardSequenceNumber, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, SetClipboardData,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, HWND_MESSAGE,
    MSG, RegisterClassW, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLIPBOARDUPDATE,
    WNDCLASSW,
};
use windows::core::{PCWSTR, w};

use super::input;
use crate::Error;

const CF_UNICODETEXT: u32 = 13;
/// Formats whose data is a GDI handle, not memory; Windows re-synthesises the useful
/// ones (bitmaps from CF_DIB) on restore.
const GDI_FORMATS: [u32; 8] = [2, 3, 9, 14, 0x80, 0x82, 0x83, 0x8E];
const MAX_FORMAT_BYTES: usize = 64 << 20;
/// How long the copy may take to land in the clipboard.
const COPY_WAIT: Duration = Duration::from_millis(700);

/// While set (a deadline in ms since the epoch), the double-copy watcher ignores updates
/// because they come from our own simulated copy and restore.
static SUPPRESS_UNTIL_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

fn os(e: windows::core::Error) -> Error {
    Error::Os(e.message())
}

/// A message-only window: the clipboard needs an owner window for SetClipboardData.
struct OwnerWindow(HWND);

impl OwnerWindow {
    fn new() -> Result<Self, Error> {
        // SAFETY: creates a message-only STATIC window, destroyed on drop.
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!(""),
                WINDOW_STYLE(0),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                None,
                None,
            )
        }
        .map_err(os)?;
        Ok(OwnerWindow(hwnd))
    }
}

impl Drop for OwnerWindow {
    fn drop(&mut self) {
        // SAFETY: we created this window on this thread.
        unsafe {
            let _ = DestroyWindow(self.0);
        }
    }
}

/// An open clipboard, closed on drop.
struct Open;

impl Open {
    fn new(owner: Option<HWND>) -> Result<Self, Error> {
        // Another app may hold the clipboard for a moment; retry briefly.
        for _ in 0..25 {
            // SAFETY: paired with CloseClipboard in Drop.
            if unsafe { OpenClipboard(owner) }.is_ok() {
                return Ok(Open);
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        Err(Error::ClipboardBusy)
    }
}

impl Drop for Open {
    fn drop(&mut self) {
        // SAFETY: the clipboard was opened by this thread.
        unsafe {
            let _ = CloseClipboard();
        }
    }
}

/// Every format on the clipboard with a copy of its bytes.
struct Snapshot {
    formats: Vec<(u32, Vec<u8>)>,
}

impl Snapshot {
    fn take() -> Result<Self, Error> {
        let _open = Open::new(None)?;
        let mut formats = Vec::new();
        let mut format = 0;
        loop {
            // SAFETY: the clipboard is open.
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                break;
            }
            if GDI_FORMATS.contains(&format) || (0x300..=0x3FF).contains(&format) {
                continue;
            }
            // SAFETY: the handle is owned by the clipboard; we only read it while locked.
            unsafe {
                let Ok(handle) = GetClipboardData(format) else {
                    continue;
                };
                if let Some(bytes) = read_global(HGLOBAL(handle.0)) {
                    formats.push((format, bytes));
                }
            }
        }
        Ok(Snapshot { formats })
    }

    fn restore(&self, owner: HWND) -> Result<(), Error> {
        let _open = Open::new(Some(owner))?;
        // SAFETY: the clipboard is open and owned by `owner`; each allocation is handed to
        // the clipboard on success and freed on failure.
        unsafe {
            EmptyClipboard().map_err(os)?;
            for (format, bytes) in &self.formats {
                set_bytes(*format, bytes);
            }
            mark_private();
        }
        Ok(())
    }
}

unsafe fn read_global(h: HGLOBAL) -> Option<Vec<u8>> {
    // SAFETY: `h` is a clipboard memory handle; we copy at most GlobalSize bytes.
    unsafe {
        let size = GlobalSize(h);
        if size == 0 || size > MAX_FORMAT_BYTES {
            return None;
        }
        let ptr = GlobalLock(h) as *const u8;
        if ptr.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
        let _ = GlobalUnlock(h);
        Some(bytes)
    }
}

unsafe fn set_bytes(format: u32, bytes: &[u8]) {
    // SAFETY: allocates, fills and hands memory to the open clipboard.
    unsafe {
        let Ok(h) = GlobalAlloc(GMEM_MOVEABLE, bytes.len().max(1)) else {
            return;
        };
        let ptr = GlobalLock(h) as *mut u8;
        if ptr.is_null() {
            let _ = GlobalFree(Some(h));
            return;
        }
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
        let _ = GlobalUnlock(h);
        if SetClipboardData(format, Some(HANDLE(h.0))).is_err() {
            let _ = GlobalFree(Some(h));
        }
    }
}

fn registered(name: PCWSTR) -> u32 {
    // SAFETY: registers (or looks up) a named clipboard format.
    unsafe { RegisterClipboardFormatW(name) }
}

/// Keeps the restored clipboard out of clipboard history and cloud clipboard.
unsafe fn mark_private() {
    let zero = 0u32.to_le_bytes();
    // SAFETY: the clipboard is open (see callers).
    unsafe {
        set_bytes(
            registered(w!("ExcludeClipboardContentFromMonitorProcessing")),
            &zero,
        );
        set_bytes(registered(w!("CanIncludeInClipboardHistory")), &zero);
        set_bytes(registered(w!("CanUploadToCloudClipboard")), &zero);
    }
}

fn read_text() -> Result<Option<String>, Error> {
    let _open = Open::new(None)?;
    // SAFETY: the clipboard is open; the text is copied out while locked.
    unsafe {
        let Ok(handle) = GetClipboardData(CF_UNICODETEXT) else {
            return Ok(None);
        };
        let Some(bytes) = read_global(HGLOBAL(handle.0)) else {
            return Ok(None);
        };
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
        Ok(Some(String::from_utf16_lossy(&units[..end])))
    }
}

/// Puts the snapshot back when dropped, so it happens even on errors.
struct RestoreOnDrop {
    snapshot: Snapshot,
    owner: OwnerWindow,
}

impl Drop for RestoreOnDrop {
    fn drop(&mut self) {
        if let Err(e) = self.snapshot.restore(self.owner.0) {
            tracing::warn!("couldn't restore the clipboard: {e}");
        }
        SUPPRESS_UNTIL_MS.store(now_ms() + 300, Ordering::SeqCst);
    }
}

/// Copies the selection with a simulated Ctrl+C and restores the clipboard.
pub fn copy_selection() -> Result<Option<String>, Error> {
    SUPPRESS_UNTIL_MS.store(now_ms() + 5_000, Ordering::SeqCst);
    let restore = RestoreOnDrop {
        snapshot: Snapshot::take()?,
        owner: OwnerWindow::new()?,
    };
    // SAFETY: reads a counter.
    let before = unsafe { GetClipboardSequenceNumber() };
    input::release_modifiers(Duration::from_millis(350));
    if !input::send_copy() {
        return Ok(None);
    }
    let deadline = Instant::now() + COPY_WAIT;
    // SAFETY: reads a counter.
    while unsafe { GetClipboardSequenceNumber() } == before {
        if Instant::now() > deadline {
            // Nothing selected, or the app can't copy (or runs as administrator).
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    // Let the app finish writing all its formats.
    std::thread::sleep(Duration::from_millis(40));
    let text = read_text()?;
    drop(restore);
    Ok(text)
}

/// Calls back with the copied text when the person presses Ctrl+C twice quickly.
pub struct DoubleCopyWatcher;

struct WatchState {
    last_at: Option<Instant>,
    last_text: Option<String>,
}

static WATCH: Mutex<WatchState> = Mutex::new(WatchState {
    last_at: None,
    last_text: None,
});
type Callback = Box<dyn Fn(String) + Send + Sync>;
static ON_DOUBLE_COPY: OnceLock<Callback> = OnceLock::new();
const DOUBLE_COPY_WINDOW: Duration = Duration::from_millis(650);

impl DoubleCopyWatcher {
    /// Starts listening (once per process).
    pub fn start(on_double_copy: impl Fn(String) + Send + Sync + 'static) -> Result<Self, Error> {
        if ON_DOUBLE_COPY.set(Box::new(on_double_copy)).is_err() {
            return Ok(DoubleCopyWatcher);
        }
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("double-copy".into())
            .spawn(move || {
                // SAFETY: a window class and message-only window owned by this thread, with
                // a standard message loop.
                unsafe {
                    let instance = GetModuleHandleW(None).unwrap_or_default();
                    let class = WNDCLASSW {
                        lpfnWndProc: Some(watch_proc),
                        hInstance: instance.into(),
                        lpszClassName: w!("TextExplainerClipboardWatch"),
                        ..Default::default()
                    };
                    RegisterClassW(&class);
                    let hwnd = CreateWindowExW(
                        WINDOW_EX_STYLE(0),
                        w!("TextExplainerClipboardWatch"),
                        w!(""),
                        WINDOW_STYLE(0),
                        0,
                        0,
                        0,
                        0,
                        Some(HWND_MESSAGE),
                        None,
                        Some(instance.into()),
                        None,
                    );
                    let ok = match hwnd {
                        Ok(h) => AddClipboardFormatListener(h).is_ok(),
                        Err(_) => false,
                    };
                    let _ = tx.send(ok);
                    if !ok {
                        return;
                    }
                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }
                }
            })
            .map_err(|e| Error::Os(e.to_string()))?;
        match rx.recv_timeout(Duration::from_secs(2)) {
            Ok(true) => Ok(DoubleCopyWatcher),
            _ => Err(Error::Os("couldn't listen for clipboard changes".into())),
        }
    }
}

unsafe extern "system" fn watch_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_CLIPBOARDUPDATE {
        on_clipboard_update();
        return LRESULT(0);
    }
    // SAFETY: default handling for every other message.
    unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
}

fn on_clipboard_update() {
    if now_ms() < SUPPRESS_UNTIL_MS.load(Ordering::SeqCst) {
        return;
    }
    // SAFETY: checks for our own privacy marker (set by the restore).
    if unsafe {
        IsClipboardFormatAvailable(registered(w!(
            "ExcludeClipboardContentFromMonitorProcessing"
        )))
    }
    .is_ok()
    {
        return;
    }
    // Only keyboard copies count (Ctrl is down), not apps writing the clipboard.
    if !input::control_down() {
        WATCH
            .lock()
            .map(|mut w| {
                *w = WatchState {
                    last_at: None,
                    last_text: None,
                }
            })
            .ok();
        return;
    }
    let text = match read_text() {
        Ok(Some(t)) if !t.trim().is_empty() => t,
        _ => return,
    };
    let Ok(mut state) = WATCH.lock() else { return };
    let now = Instant::now();
    let is_double = state
        .last_at
        .is_some_and(|t| now.duration_since(t) <= DOUBLE_COPY_WINDOW)
        && state.last_text.as_deref() == Some(text.as_str());
    if is_double {
        *state = WatchState {
            last_at: None,
            last_text: None,
        };
        drop(state);
        if let Some(cb) = ON_DOUBLE_COPY.get() {
            cb(text);
        }
    } else {
        *state = WatchState {
            last_at: Some(now),
            last_text: Some(text),
        };
    }
}
