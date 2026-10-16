; Hooks into Tauri's NSIS installer (bundle > windows > nsis > installerHooks).

; The program file was text-explainer.exe until 16 October 2026, when it became Plainly.exe.
; The installer closes a running copy before replacing files, but looks for the new name,
; so a copy still running under the old name is closed here (taskkill only reaches this
; user's processes in a per-user install; nothing happens when none is running). The
; installer deletes the old file itself once the new one is in place.
!macro NSIS_HOOK_PREINSTALL
  nsExec::Exec 'taskkill /F /IM text-explainer.exe'
  Pop $0
  Sleep 500
!macroend
