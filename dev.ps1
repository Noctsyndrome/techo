param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]] $CargoArgs = @("run")
)

# Load the MSVC linker and Windows SDK that the Rust MSVC target requires,
# then forward all arguments to Cargo. Example: .\dev.ps1 test
$vsDevCmd = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"
if (-not (Test-Path $vsDevCmd)) {
    throw "Visual C++ Build Tools were not found at $vsDevCmd"
}

cmd.exe /c "call `"$vsDevCmd`" -arch=x64 -host_arch=x64 >nul && set" |
    ForEach-Object {
        $pair = $_ -split "=", 2
        if ($pair.Count -eq 2) {
            Set-Item -Path "Env:$($pair[0])" -Value $pair[1]
        }
    }

$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
$env:Path = "$cargoBin;$env:Path"
Set-Location -LiteralPath $PSScriptRoot
$forwardArgs = $CargoArgs
if ($CargoArgs.Count -gt 1 -and $CargoArgs[0] -eq "clippy" -and $CargoArgs -notcontains "--") {
    # PowerShell consumes a bare `--` when invoking a script. Restore the
    # separator Cargo needs before forwarding Clippy lints such as `-D warnings`.
    $lintIndex = -1
    for ($i = 1; $i -lt $CargoArgs.Count; $i++) {
        if ($CargoArgs[$i] -match '^-[DAWF]') { $lintIndex = $i; break }
    }
    if ($lintIndex -gt 0) {
        $forwardArgs = @($CargoArgs[0..($lintIndex - 1)]) + @("--") + @($CargoArgs[$lintIndex..($CargoArgs.Count - 1)])
    }
}
if ($CargoArgs.Count -eq 1 -and $CargoArgs[0] -eq "run") {
    # Keep checkout journals accessible while installed techo uses a stable home directory.
    $forwardArgs = @("run", "--", "--data-dir", (Join-Path $PSScriptRoot "logs"))
}

& cargo @forwardArgs
exit $LASTEXITCODE
