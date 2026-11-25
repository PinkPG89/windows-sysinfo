# Per-user installer for sysmon-tray (no admin rights required).
$ErrorActionPreference = 'Stop'

$AppName   = 'SysmonTray'
$ExeName   = 'sysmon-tray.exe'
$InstallDir = Join-Path $env:LOCALAPPDATA "Programs\$AppName"
$SrcDir    = Split-Path -Parent $MyInvocation.MyCommand.Path
$ExePath   = Join-Path $InstallDir $ExeName

Write-Host "Installing $AppName to $InstallDir ..."

# Stop a running instance so the file can be replaced.
Get-Process -Name ($ExeName -replace '\.exe$','') -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500

New-Item -ItemType Directory -Force $InstallDir | Out-Null
Copy-Item (Join-Path $SrcDir $ExeName) $ExePath -Force
Copy-Item (Join-Path $SrcDir 'uninstall.ps1') (Join-Path $InstallDir 'uninstall.ps1') -Force

# Start automatically at sign-in.
Set-ItemProperty -Path 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run' -Name $AppName -Value "`"$ExePath`""

# Start Menu shortcut.
$lnk = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$AppName.lnk"
$sh = New-Object -ComObject WScript.Shell
$s = $sh.CreateShortcut($lnk)
$s.TargetPath = $ExePath
$s.WorkingDirectory = $InstallDir
$s.Save()

# Entry in Settings > Apps (uninstall).
$uk = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$AppName"
New-Item -Path $uk -Force | Out-Null
Set-ItemProperty $uk -Name DisplayName     -Value 'Sysmon Tray'
Set-ItemProperty $uk -Name DisplayVersion  -Value '0.1.0'
Set-ItemProperty $uk -Name Publisher       -Value 'sysmon-tray'
Set-ItemProperty $uk -Name InstallLocation -Value $InstallDir
Set-ItemProperty $uk -Name DisplayIcon     -Value $ExePath
Set-ItemProperty $uk -Name UninstallString -Value "powershell.exe -NoProfile -ExecutionPolicy Bypass -File `"$InstallDir\uninstall.ps1`""
Set-ItemProperty $uk -Name NoModify -Value 1 -Type DWord
Set-ItemProperty $uk -Name NoRepair -Value 1 -Type DWord

Start-Process $ExePath
Write-Host 'Done. Look for the CPU/RAM icons in the taskbar tray.'
