# Rebuild techo and reopen it in a fresh Windows Terminal window. The previous
# techo is stopped first; its host shell (dev-host.ps1) then exits cleanly and
# Windows Terminal closes that tab by itself.
# Usage: .\scripts\dev-window.ps1 [-DataDir path]
param(
    [string] $DataDir = ""
)

$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if ($DataDir -eq "") { $DataDir = Join-Path $root "logs" }
$exe = Join-Path $root "target\debug\techo.exe"
$host_script = Join-Path $PSScriptRoot "dev-host.ps1"

Get-Process techo -ErrorAction SilentlyContinue | Stop-Process -Force
# A host still waiting at a crash prompt from an earlier round is closed too.
Get-CimInstance Win32_Process -Filter "Name = 'pwsh.exe'" |
    Where-Object { $_.ProcessId -ne $PID -and $_.CommandLine -and $_.CommandLine.Contains("dev-host.ps1") } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
Start-Sleep -Milliseconds 300

& (Join-Path $root "dev.ps1") build --locked
if ($LASTEXITCODE) { exit $LASTEXITCODE }

# Claude's tool shell carries NO_COLOR=1, which crossterm honours; the app should
# see the terminal as the user does.
Remove-Item Env:NO_COLOR -ErrorAction SilentlyContinue
Start-Process wt -ArgumentList "-d", $root, "pwsh", "-File", $host_script, "-Exe", $exe, "-DataDir", $DataDir
Start-Sleep -Seconds 2
Get-Process techo -ErrorAction SilentlyContinue | Select-Object Id, StartTime
