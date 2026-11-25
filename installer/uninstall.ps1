$ErrorActionPreference = 'SilentlyContinue'

$AppName    = 'SysmonTray'
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\$AppName"

Get-Process -Name 'sysmon-tray' | Stop-Process -Force
Start-Sleep -Milliseconds 500

Remove-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name $AppName
Remove-Item "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$AppName" -Recurse -Force
Remove-Item (Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$AppName.lnk") -Force

# This script lives inside the install dir, so delete the folder from a detached process.
Start-Process powershell.exe -WindowStyle Hidden -ArgumentList "-NoProfile -Command Start-Sleep 2; Remove-Item -LiteralPath '$InstallDir' -Recurse -Force"
Write-Host 'Uninstalled.'
