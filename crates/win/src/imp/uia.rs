//! UI Automation: ask the focused control for its selected text, where it is on screen,
//! and (for single words) the paragraph around it. No clipboard, no keystrokes.

use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoUninitialize,
    SAFEARRAY,
};
use windows::Win32::System::Ole::{
    SafeArrayAccessData, SafeArrayDestroy, SafeArrayGetLBound, SafeArrayGetUBound,
    SafeArrayUnaccessData,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationTextPattern, IUIAutomationTextRange,
    TextUnit_Paragraph, UIA_TextPatternId,
};

use super::os_err;
use crate::{Error, Method, Rect, Selection};

/// The most text read through accessibility (a few pages).
const MAX_CHARS: i32 = 20_000;
/// Context is only gathered for short selections (words and phrases).
const CONTEXT_FOR_CHARS: usize = 60;

struct ComGuard;
impl Drop for ComGuard {
    fn drop(&mut self) {
        // SAFETY: paired with the successful CoInitializeEx below on this thread.
        unsafe { CoUninitialize() };
    }
}

pub fn selection() -> Result<Option<Selection>, Error> {
    // SAFETY: plain COM calls on this thread; every interface is released on drop.
    unsafe {
        let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
        let _guard = hr.is_ok().then_some(ComGuard);
        let uia: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).map_err(os_err)?;
        let focused = uia.GetFocusedElement().map_err(os_err)?;
        let Ok(pattern) =
            focused.GetCurrentPatternAs::<IUIAutomationTextPattern>(UIA_TextPatternId)
        else {
            return Ok(None);
        };
        let ranges = pattern.GetSelection().map_err(os_err)?;
        let count = ranges.Length().map_err(os_err)?;
        let mut text = String::new();
        let mut bounds: Option<Rect> = None;
        let mut first: Option<IUIAutomationTextRange> = None;
        for i in 0..count {
            let range = ranges.GetElement(i).map_err(os_err)?;
            let part = range.GetText(MAX_CHARS).map_err(os_err)?.to_string();
            if part.is_empty() {
                continue;
            }
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&part);
            if let Some(r) = range_bounds(&range) {
                bounds = Some(bounds.map_or(r, |b| b.union(&r)));
            }
            if first.is_none() {
                first = Some(range);
            }
        }
        if text.trim().is_empty() {
            return Ok(None);
        }
        let context = match &first {
            Some(range) if text.chars().count() <= CONTEXT_FOR_CHARS => paragraph_around(range),
            _ => None,
        };
        Ok(Some(Selection {
            text,
            context,
            bounds,
            method: Method::Accessibility,
        }))
    }
}

/// The paragraph containing the selection, for explaining a word in context.
unsafe fn paragraph_around(range: &IUIAutomationTextRange) -> Option<String> {
    // SAFETY: COM calls on a live range; the clone is released on drop.
    unsafe {
        let wider = range.Clone().ok()?;
        wider.ExpandToEnclosingUnit(TextUnit_Paragraph).ok()?;
        let text = wider.GetText(2_000).ok()?.to_string();
        let text = text.trim().to_string();
        (!text.is_empty()).then_some(text)
    }
}

/// The union of the rectangles a text range covers on screen.
unsafe fn range_bounds(range: &IUIAutomationTextRange) -> Option<Rect> {
    // SAFETY: the SAFEARRAY returned by UIA is ours to read and destroy; it holds
    // doubles in groups of four (left, top, width, height).
    unsafe {
        let array: *mut SAFEARRAY = range.GetBoundingRectangles().ok()?;
        if array.is_null() {
            return None;
        }
        let result = read_rects(array);
        let _ = SafeArrayDestroy(array);
        result
    }
}

unsafe fn read_rects(array: *mut SAFEARRAY) -> Option<Rect> {
    // SAFETY: see `range_bounds`.
    unsafe {
        let lo = SafeArrayGetLBound(array, 1).ok()?;
        let hi = SafeArrayGetUBound(array, 1).ok()?;
        let n = (hi - lo + 1).max(0) as usize;
        if n < 4 {
            return None;
        }
        let mut data: *mut core::ffi::c_void = core::ptr::null_mut();
        SafeArrayAccessData(array, &mut data).ok()?;
        let values = core::slice::from_raw_parts(data as *const f64, n);
        let mut out: Option<Rect> = None;
        for q in values.chunks_exact(4) {
            let r = Rect::new(
                q[0] as i32,
                q[1] as i32,
                (q[0] + q[2]) as i32,
                (q[1] + q[3]) as i32,
            );
            if r.width() > 0 || r.height() > 0 {
                out = Some(out.map_or(r, |o| o.union(&r)));
            }
        }
        let _ = SafeArrayUnaccessData(array);
        out
    }
}
