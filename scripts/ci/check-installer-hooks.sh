#!/bin/sh
# Compiles src-tauri/windows/installer-hooks.nsh inside a small stand-in for Tauri's
# installer script (same variable and define names), so a mistake in the hooks shows up
# here instead of breaking the Windows build. Needs makensis (apt install nsis).
set -e
dir="$(cd "$(dirname "$0")/../.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
cat > "$work/test.nsi" <<NSI
Unicode true
!include "LogicLib.nsh"
!define BUNDLEID "dev.omikhan.textexplainer"
!define PRODUCTNAME "Plainly"
Var DeleteAppDataCheckboxState
!include "$dir/src-tauri/windows/installer-hooks.nsh"
OutFile "$work/test.exe"
Section Install
  !insertmacro NSIS_HOOK_PREINSTALL
  WriteUninstaller "\$INSTDIR\uninstall.exe"
SectionEnd
Section Uninstall
  StrCpy \$DeleteAppDataCheckboxState 0
  !insertmacro NSIS_HOOK_POSTUNINSTALL
SectionEnd
NSI
makensis -V2 "$work/test.nsi"
