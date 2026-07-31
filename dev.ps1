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
$forwardArgs = $CargoArgs
if ($CargoArgs.Count -gt 1 -and $CargoArgs[0] -eq "clippy" -and $CargoArgs[1].StartsWith("-")) {
    # PowerShell consumes a bare `--` when invoking a script. Restore the
    # separator Cargo needs before forwarding Clippy lints such as `-D warnings`.
    $forwardArgs = @($CargoArgs[0], "--") + $CargoArgs[1..($CargoArgs.Count - 1)]
}

& cargo @forwardArgs
exit $LASTEXITCODE
