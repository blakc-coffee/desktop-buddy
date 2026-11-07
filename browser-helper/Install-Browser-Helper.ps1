param(
    [Parameter(Mandatory=$true)]
    [ValidatePattern('^[a-p]{32}$')][string]$ExtensionId,
    [ValidateSet('Chrome','Edge','Both')][string]$Browser = 'Both'
)
$ErrorActionPreference = 'Stop'
$taskHost = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../buddy-browser-host.exe'))
if (!(Test-Path -LiteralPath $taskHost -PathType Leaf)) { throw 'Keep browser-helper beside buddy-browser-host.exe in the complete release folder.' }
$taskManifest = Join-Path $PSScriptRoot "page-helper-host-$ExtensionId.json"
$taskHostConfig = @{
    name = 'local.dharu.desktopbuddy.page_helper'
    description = 'Desktop Buddy page helper'
    path = $taskHost
    type = 'stdio'
    allowed_origins = @("chrome-extension://$ExtensionId/")
} | ConvertTo-Json
[IO.File]::WriteAllText($taskManifest, $taskHostConfig, [Text.UTF8Encoding]::new($false))
$taskBrowsers = if ($Browser -eq 'Both') { @('Google/Chrome','Microsoft/Edge') } elseif ($Browser -eq 'Chrome') { @('Google/Chrome') } else { @('Microsoft/Edge') }
foreach ($taskBrowser in $taskBrowsers) {
    $taskRegistry = "HKCU:/Software/$taskBrowser/NativeMessagingHosts/local.dharu.desktopbuddy.page_helper"
    New-Item -Path $taskRegistry -Force | Out-Null
    Set-Item -LiteralPath $taskRegistry -Value $taskManifest
}
Write-Output 'Registered this browser extension for the current Windows user. Click Connect this page in the helper.'
