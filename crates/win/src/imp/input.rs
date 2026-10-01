//! Synthesising Ctrl+C safely while the person may still be holding the hotkey.

use std::time::{Duration, Instant};

use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBD_EVENT_FLAGS, KEYBDINPUT,
    KEYEVENTF_KEYUP, SendInput, VIRTUAL_KEY, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
};

/// Marks our own synthetic key events.
pub const INJECTED_MARKER: usize = 0x5445_5850; // "TEXP"
/// An unassigned virtual key: pressing it while Alt or Win is down stops their release
/// from opening the app's menu bar or the Start menu.
const VK_UNASSIGNED: VIRTUAL_KEY = VIRTUAL_KEY(0xE8);
const VK_C: VIRTUAL_KEY = VIRTUAL_KEY(0x43);
const MODIFIERS: [VIRTUAL_KEY; 5] = [VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN];

fn is_down(vk: VIRTUAL_KEY) -> bool {
    // SAFETY: reads global key state; no pointers.
    unsafe { GetAsyncKeyState(vk.0 as i32) < 0 }
}

fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    KEYBD_EVENT_FLAGS(0)
                },
                time: 0,
                dwExtraInfo: INJECTED_MARKER,
            },
        },
    }
}

fn send(inputs: &[INPUT]) -> bool {
    // SAFETY: `inputs` is a valid slice of INPUT structures.
    let sent = unsafe { SendInput(inputs, std::mem::size_of::<INPUT>() as i32) };
    sent as usize == inputs.len()
}

/// Waits briefly for the person to let go of the hotkey's modifiers; any still held are
/// released synthetically so the copy is a plain Ctrl+C (Ctrl+Shift+C opens developer
/// tools in browsers).
pub fn release_modifiers(wait: Duration) {
    let deadline = Instant::now() + wait;
    while MODIFIERS.iter().any(|&vk| is_down(vk)) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    let held: Vec<VIRTUAL_KEY> = MODIFIERS.into_iter().filter(|&vk| is_down(vk)).collect();
    if held.is_empty() {
        return;
    }
    let mut inputs = Vec::new();
    if held
        .iter()
        .any(|&vk| vk == VK_MENU || vk == VK_LWIN || vk == VK_RWIN)
    {
        inputs.push(key(VK_UNASSIGNED, false));
        inputs.push(key(VK_UNASSIGNED, true));
    }
    inputs.extend(held.iter().map(|&vk| key(vk, true)));
    send(&inputs);
}

/// Presses Ctrl+C in the focused app.
pub fn send_copy() -> bool {
    send(&[
        key(VK_CONTROL, false),
        key(VK_C, false),
        key(VK_C, true),
        key(VK_CONTROL, true),
    ])
}

/// True while a Ctrl key is physically down (the double-copy trigger checks this).
pub fn control_down() -> bool {
    is_down(VK_CONTROL)
}
