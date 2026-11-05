$ErrorActionPreference = 'Stop'
$taskProject = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $taskProject
$taskTools = Join-Path (Split-Path -Parent $taskProject) 'tools'
if (Test-Path -LiteralPath (Join-Path $taskTools 'cargo/bin/cargo.exe')) {
    $env:CARGO_HOME = Join-Path $taskTools 'cargo'
    $env:RUSTUP_HOME = Join-Path $taskTools 'rustup'
    $env:PATH = "$env:CARGO_HOME/bin;$env:PATH"
}
$env:CARGO_BUILD_JOBS = '2'
$env:NODE_USE_SYSTEM_CA = '1'
npm.cmd run tauri -- build --no-bundle
if ($LASTEXITCODE -ne 0) { throw 'Desktop build failed.' }
