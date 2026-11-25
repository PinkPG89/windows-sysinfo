; Inno Setup script - per-user installer (no admin rights required)
; Build:  ISCC.exe installer\sysmon-tray.iss   (or run build-dist.ps1)

#define AppName "Sysmon Tray"
#define AppVersion "0.1.0"
#define AppExe "sysmon-tray.exe"

[Setup]
AppId={{B7C3A5E2-6D41-4F0A-9E58-3C1F2A7D9B10}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=sysmon-tray
DefaultDirName={localappdata}\Programs\SysmonTray
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename=SysmonTray-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
UninstallDisplayIcon={app}\{#AppExe}
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion

[InstallDelete]
; Leftover from the old script-based installer
Type: files; Name: "{app}\uninstall.ps1"

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"

[Registry]
; Start at sign-in
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "SysmonTray"; ValueData: """{app}\{#AppExe}"""; Flags: uninsdeletevalue
; Remove the entry left by the old install.bat/zip installer, if present
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Uninstall\SysmonTray"; Flags: dontcreatekey deletekey

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{sys}\taskkill.exe"; Parameters: "/F /IM {#AppExe}"; Flags: runhidden; RunOnceId: "KillSysmonTray"

[Code]
// Stop a running instance so the exe can be replaced.
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#AppExe}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Sleep(500);
  Result := '';
end;
