; Hooks into Tauri's NSIS installer (bundle > windows > nsis > installerHooks).
; Compile-checked by scripts/ci/check-installer-hooks.sh (needs makensis).

!include "FileFunc.nsh"

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

; Downloaded models live in models\ next to the app (so they go on the drive the person
; installed to), and for installs from before 16 October 2026 in the app data folder.
; "Delete the application data" deletes them with everything else. Otherwise the person
; is told how much stays and may delete it too: gigabytes left behind without a word would
; look like bloat (the owner's call). Silent uninstalls (reinstalling) keep them.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $DeleteAppDataCheckboxState = 1
    RmDir /r "$INSTDIR\models"
  ${Else}
    StrCpy $R5 0
    ${If} ${FileExists} "$INSTDIR\models\*.*"
      ${GetSize} "$INSTDIR\models" "/S=0M /G=1" $R6 $R7 $R8
      IntOp $R5 $R5 + $R6
    ${EndIf}
    ${If} ${FileExists} "$LOCALAPPDATA\${BUNDLEID}\models\*.*"
      ${GetSize} "$LOCALAPPDATA\${BUNDLEID}\models" "/S=0M /G=1" $R6 $R7 $R8
      IntOp $R5 $R5 + $R6
    ${EndIf}
    ${If} $R5 > 0
      IfSilent plainly_models_done
      ; Megabytes to gigabytes with one decimal.
      IntOp $R6 $R5 / 1024
      IntOp $R7 $R5 % 1024
      IntOp $R7 $R7 * 10
      IntOp $R7 $R7 / 1024
      MessageBox MB_YESNO|MB_ICONQUESTION|MB_DEFBUTTON2 "The AI models you downloaded ($R6.$R7 GB) are still on this computer, so reinstalling Plainly won't download them again.$\n$\nDelete them too?" IDNO plainly_models_done
      RmDir /r "$INSTDIR\models"
      RmDir /r "$LOCALAPPDATA\${BUNDLEID}\models"
    ${EndIf}
  ${EndIf}
  plainly_models_done:
  ; The installer removed the app's own files; this is empty now unless models stayed.
  RmDir "$INSTDIR"
!macroend
