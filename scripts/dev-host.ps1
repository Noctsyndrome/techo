# The shell that hosts techo in a dev window. It exits 0 once techo ends, so the
# terminal tab closes on its own, and stays only when techo crashed, so the
# panic can be read. Launched by dev-window.ps1.
param(
    [string] $Exe,
    [string] $DataDir
)

& $Exe --data-dir $DataDir
$code = $LASTEXITCODE
# -1 / 0xffffffff is a Stop-Process from dev-window.ps1, not a crash.
if ($code -ne 0 -and $code -ne -1 -and $code -ne 4294967295) {
    Read-Host "techo exited with code $code. Enter closes this window"
}
exit 0
