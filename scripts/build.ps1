param(
    [string]$OutputDirectory
)

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
$buildStarted = Get-Date
npm.cmd run tauri -- build --no-bundle
if ($LASTEXITCODE -ne 0) { throw 'Desktop build failed.' }

$indexPath = Join-Path $taskProject 'dist/index.html'
if (!(Test-Path -LiteralPath $indexPath)) { throw 'Packaged frontend entry point is missing.' }
$assets = Get-ChildItem -LiteralPath (Join-Path $taskProject 'dist/assets') -File
if (!$assets -or !(($assets | Where-Object Extension -eq '.js'))) {
    throw 'Packaged frontend JavaScript is missing.'
}
if ((Get-Item -LiteralPath $indexPath).LastWriteTime -lt $buildStarted) {
    throw 'The frontend was not rebuilt by this release command.'
}
$metadataJson = cargo metadata --no-deps --format-version 1 --manifest-path src-tauri/Cargo.toml
if ($LASTEXITCODE -ne 0) { throw 'Could not resolve the Cargo release directory.' }
$metadata = $metadataJson | ConvertFrom-Json
$releasePath = Join-Path $metadata.target_directory 'release/desktop-buddy.exe'
$release = Get-Item -LiteralPath $releasePath
if ($release.LastWriteTime -lt $buildStarted) { throw 'The Windows executable was not rebuilt.' }

if ($OutputDirectory) {
    $destination = New-Item -ItemType Directory -Force -Path $OutputDirectory
    $releasePath = Join-Path $destination.FullName 'desktop-buddy.exe'
    if ([IO.Path]::GetFullPath($releasePath) -ne $release.FullName) {
        Copy-Item -LiteralPath $release.FullName -Destination $releasePath -Force
    }
    $taskBrowserHost = Join-Path $metadata.target_directory 'release/buddy-browser-host.exe'
    if (!(Test-Path -LiteralPath $taskBrowserHost)) { throw 'Browser helper executable is missing.' }
    Copy-Item -LiteralPath $taskBrowserHost -Destination (Join-Path $destination.FullName 'buddy-browser-host.exe') -Force
    Copy-Item -LiteralPath (Join-Path $taskProject 'browser-helper') -Destination $destination.FullName -Recurse -Force
}
[pscustomobject]@{
    Executable = $releasePath
    SHA256 = (Get-FileHash -LiteralPath $releasePath -Algorithm SHA256).Hash
    BuiltAt = (Get-Item -LiteralPath $releasePath).LastWriteTime
}
