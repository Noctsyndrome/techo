# Rebuild techo and reopen it in a fresh Windows Terminal window, closing the
# previous one first (both the app and the shell that keeps its tab open).
# Usage: .\scripts\dev-window.ps1 [-DataDir path]
param(
    [string] $DataDir = ""
)

$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if ($DataDir -eq "") { $DataDir = Join-Path $root "logs" }
$exe = Join-Path $root "target\debug\techo.exe"

# The app first, then the shell hosting it. Match the exact launch line this
# script writes, never this script's own shell or whoever invoked it.
$launch = "& '$exe' --data-dir"
Get-CimInstance Win32_Process -Filter "Name = 'techo.exe' OR Name = 'pwsh.exe'" |
    Where-Object {
        $_.ProcessId -ne $PID -and
        (($_.Name -eq "techo.exe") -or ($_.CommandLine -and $_.CommandLine.Contains($launch)))
    } |
    Sort-Object { $_.Name -ne "techo.exe" } |
    ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }
Start-Sleep -Milliseconds 300

& (Join-Path $root "dev.ps1") build --locked
if ($LASTEXITCODE) { exit $LASTEXITCODE }

# Claude's tool shell carries NO_COLOR=1, which crossterm honours; the app should
# see the terminal as the user does.
Remove-Item Env:NO_COLOR -ErrorAction SilentlyContinue
Start-Process wt -ArgumentList "-d", $root, "pwsh", "-NoExit", "-Command", "& '$exe' --data-dir '$DataDir'"
Start-Sleep -Seconds 2
Get-Process techo -ErrorAction SilentlyContinue | Select-Object Id, StartTime
