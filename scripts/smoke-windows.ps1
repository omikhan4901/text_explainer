# Installs the built NSIS installer silently, checks the bundled model engine runs, then
# starts the app and waits for its "started" log line. Catches what unit tests can't:
# a missing DLL or resource in the bundle, or a panic during startup.
# Runs on the Windows CI job after `tauri build`.
$ErrorActionPreference = "Stop"

$installer = Get-ChildItem "target/release/bundle/nsis/*.exe" | Select-Object -First 1
if (-not $installer) { throw "No installer in target/release/bundle/nsis" }
Write-Host "Installing $($installer.Name)"
$p = Start-Process $installer.FullName -ArgumentList "/S" -Wait -PassThru
if ($p.ExitCode -ne 0) { throw "The installer exited with $($p.ExitCode)" }

# Per-user install: usually %LOCALAPPDATA%\Text Explainer; otherwise find it by the engine.
$server = Get-Item (Join-Path $env:LOCALAPPDATA "Text Explainer\resources\llama\llama-server.exe") -ErrorAction SilentlyContinue
if (-not $server) {
  $server = Get-ChildItem $env:LOCALAPPDATA -Recurse -Depth 4 -Filter "llama-server.exe" -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -like "*\resources\llama\*" } | Select-Object -First 1
}
if (-not $server) { throw "llama-server.exe isn't in the installed app" }
$installDir = $server.Directory.Parent.Parent.FullName
Write-Host "Installed in $installDir"
foreach ($f in "resources\dictionary.sqlite", "resources\WORDNET-LICENSE.txt") {
  if (-not (Test-Path (Join-Path $installDir $f))) { throw "$f is missing from the install" }
}

# llama-server prints its version on stderr; don't let that count as an error.
$ErrorActionPreference = "Continue"
$version = & $server.FullName --version 2>&1 | Out-String
$ErrorActionPreference = "Stop"
if ($LASTEXITCODE -ne 0) { throw "llama-server --version failed ($LASTEXITCODE): $version" }
Write-Host "Engine: $($version.Trim())"

$app = Get-ChildItem $installDir -Filter "*.exe" | Where-Object { $_.Name -notlike "uninstall*" } | Select-Object -First 1
if (-not $app) { throw "No app executable in $installDir" }
$log = Join-Path $env:LOCALAPPDATA "dev.omikhan.textexplainer\logs\app.log"
Remove-Item $log -ErrorAction SilentlyContinue

# Memory of the app plus its WebView2 processes. Reported, not enforced: runners vary.
function Measure-AppMemory($procId, $label) {
  $all = Get-CimInstance Win32_Process
  $tree = @($procId)
  do {
    $more = $all | Where-Object { $tree -contains $_.ParentProcessId -and $tree -notcontains $_.ProcessId }
    $tree += @($more | ForEach-Object { $_.ProcessId })
  } while ($more)
  $procs = $tree | ForEach-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue }
  $ws = ($procs | Measure-Object WorkingSet64 -Sum).Sum / 1MB
  $private = ($procs | Measure-Object PrivateMemorySize64 -Sum).Sum / 1MB
  Write-Host ("Memory without a model, {0}: {1:N0} MB private, {2:N0} MB working set, {3} processes" -f $label, $private, $ws, $procs.Count)
}

# Starts the app, waits for its "started" line, checks it, and measures it.
function Test-AppStart($arguments, $label) {
  Remove-Item $log -ErrorAction SilentlyContinue
  Write-Host "Starting $($app.Name) $arguments ($label)"
  $proc = if ($arguments) { Start-Process $app.FullName -ArgumentList $arguments -PassThru } else { Start-Process $app.FullName -PassThru }
  try {
    $deadline = (Get-Date).AddSeconds(60)
    $line = $null
    while ((Get-Date) -lt $deadline -and -not $line) {
      if ($proc.HasExited) { throw "The app exited during startup with $($proc.ExitCode)" }
      if (Test-Path $log) {
        $line = Select-String -Path $log -Pattern "Text Explainer started" | Select-Object -First 1
      }
      if (-not $line) { Start-Sleep -Milliseconds 500 }
    }
    if (-not $line) { throw "No 'started' line in $log within 60 s" }
    Write-Host $line.Line
    foreach ($want in "engine_found=true", "dictionary=true") {
      if ($line.Line -notmatch [regex]::Escape($want)) { throw "Expected $want" }
    }
    # Still running a few seconds later (no delayed panic in the tray or watchers).
    Start-Sleep -Seconds 5
    if ($proc.HasExited) { throw "The app exited after startup with $($proc.ExitCode)" }
    Measure-AppMemory $proc.Id $label
  } finally {
    if (Test-Path $log) { Write-Host "--- app.log ---"; Get-Content $log | Write-Host }
    if (-not $proc.HasExited) { Stop-Process -Id $proc.Id -Force }
    Get-Process msedgewebview2 -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
  }
}

# First launch: the main window opens on the first-run screen.
Test-AppStart $null "main window open"

# After setup, started at login: only the tray (how the app spends most of its time).
$config = Join-Path $env:APPDATA "dev.omikhan.textexplainer"
New-Item -ItemType Directory -Force $config | Out-Null
Set-Content (Join-Path $config "settings.json") '{"first_run_done": true, "start_with_windows": false}'
Test-AppStart "--autostart" "in the tray after login"

Write-Host "Smoke test passed"
