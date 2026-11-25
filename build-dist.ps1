# Build the release exe and create dist\SysmonTray-Setup.exe (Inno Setup)
# Usage (from project root):  powershell -ExecutionPolicy Bypass -File .\build-dist.ps1
# Requires: Rust, VS Build Tools, Inno Setup 6  (winget install JRSoftware.InnoSetup)
$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$env:Path += ";$env:USERPROFILE\.cargo\bin"

cargo build --release
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

$iscc = @(
  "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
  "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
  "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { throw 'Inno Setup 6 not found. Install: winget install JRSoftware.InnoSetup' }

Remove-Item (Join-Path $PSScriptRoot 'dist') -Recurse -Force -ErrorAction SilentlyContinue
& $iscc 'installer\sysmon-tray.iss'
if ($LASTEXITCODE -ne 0) { throw 'Inno Setup compile failed' }

Write-Host "Done: $(Join-Path $PSScriptRoot 'dist\SysmonTray-Setup.exe')"
