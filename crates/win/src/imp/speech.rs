//! Reading a word aloud with the Windows voice (SAPI). Offline: Windows' own voices.

use std::sync::OnceLock;
use std::sync::mpsc::{self, Sender};

use windows::Win32::Media::Speech::{
    ISpVoice, SPF_ASYNC, SPF_IS_NOT_XML, SPF_PURGEBEFORESPEAK, SpVoice,
};
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::core::PCWSTR;

use crate::Error;

/// The voice lives on its own thread for the whole run; texts are sent to it.
static VOICE: OnceLock<Result<Sender<Vec<u16>>, String>> = OnceLock::new();

/// Speaks `text` with the default Windows voice, stopping whatever it was saying. Returns
/// at once; the text is read as plain text (never as SAPI XML markup).
pub fn speak(text: &str) -> Result<(), Error> {
    let voice = VOICE
        .get_or_init(start)
        .as_ref()
        .map_err(|e| Error::Os(e.clone()))?;
    let wide: Vec<u16> = text
        .encode_utf16()
        .filter(|&c| c != 0)
        .chain(Some(0))
        .collect();
    voice
        .send(wide)
        .map_err(|_| Error::Os("the voice stopped".into()))
}

fn start() -> Result<Sender<Vec<u16>>, String> {
    let (tx, rx) = mpsc::channel::<Vec<u16>>();
    let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), String>>(1);
    std::thread::Builder::new()
        .name("te-voice".into())
        .spawn(move || {
            // SAFETY: COM is initialised on this thread, which owns the voice and outlives
            // every call made on it.
            let voice: ISpVoice = unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                match CoCreateInstance(&SpVoice, None, CLSCTX_ALL) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e.message()));
                        return;
                    }
                }
            };
            let _ = ready_tx.send(Ok(()));
            let flags = (SPF_ASYNC.0 | SPF_PURGEBEFORESPEAK.0 | SPF_IS_NOT_XML.0) as u32;
            // The text being spoken stays alive until the next one purges it.
            let mut _speaking: Vec<u16>;
            for text in rx {
                // SAFETY: `text` is NUL-terminated UTF-16 and is kept alive (in
                // `_speaking`) while the voice may still read it.
                if let Err(e) = unsafe { voice.Speak(PCWSTR(text.as_ptr()), flags, None) } {
                    tracing::warn!("couldn't speak: {}", e.message());
                }
                _speaking = text;
            }
        })
        .map_err(|e| e.to_string())?;
    ready_rx
        .recv()
        .map_err(|_| "the voice couldn't start".to_string())??;
    Ok(tx)
}
