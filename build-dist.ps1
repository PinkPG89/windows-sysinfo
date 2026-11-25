# Build the release exe and create dist\SysmonTray-Setup.zip
# Usage (from project root):  powershell -ExecutionPolicy Bypass -File .\build-dist.ps1
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$env:Path += ";$env:USERPROFILE\.cargo\bin"

cargo build --release
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

$dist = Join-Path $PSScriptRoot 'dist\SysmonTray-Setup'
Remove-Item (Join-Path $PSScriptRoot 'dist') -Recurse -Force -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $dist | Out-Null

Copy-Item 'target\release\sysmon-tray.exe' $dist
Copy-Item 'installer\*' $dist

$zip = Join-Path $PSScriptRoot 'dist\SysmonTray-Setup.zip'
Compress-Archive -Path "$dist\*" -DestinationPath $zip -Force

Write-Host "Done: $zip"
