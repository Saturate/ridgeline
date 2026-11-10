; Built by scripts/package.sh; AppVersion, SourceExe and IconFile come in as /D defines.
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
; Fixed AppId so upgrades replace the existing install instead of adding a second one.
AppId={{6F1C3B52-8E2A-4C4B-9C61-5B0D7A2E9F10}
AppName=Ridgeline
AppVersion={#AppVersion}
AppPublisher=Saturate
DefaultDirName={autopf}\Ridgeline
DefaultGroupName=Ridgeline
DisableProgramGroupPage=yes
; Per-user like the Tauri NSIS installer it replaces, so upgrades land in the
; same scope and no admin prompt appears.
PrivilegesRequired=lowest
SetupIconFile={#IconFile}
UninstallDisplayIcon={app}\ridgeline.exe
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
Compression=lzma2
SolidCompression=yes
WizardStyle=modern

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Ridgeline"; Filename: "{app}\ridgeline.exe"
Name: "{autodesktop}\Ridgeline"; Filename: "{app}\ridgeline.exe"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; Flags: unchecked

[Run]
Filename: "{app}\ridgeline.exe"; Description: "{cm:LaunchProgram,Ridgeline}"; Flags: nowait postinstall skipifsilent

[Code]
const
  TauriUninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\Ridgeline';

// Versions up to 0.7 were Tauri NSIS installs with their own uninstall entry.
// Remove that silently first so upgrading doesn't leave two copies installed.
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Uninstaller, InstallDir: String;
  ResultCode: Integer;
begin
  Result := '';
  if not RegQueryStringValue(HKCU, TauriUninstallKey, 'UninstallString', Uninstaller) then
    exit;
  Uninstaller := RemoveQuotes(Uninstaller);
  if RegQueryStringValue(HKCU, TauriUninstallKey, 'InstallLocation', InstallDir) then
    InstallDir := RemoveQuotes(InstallDir)
  else
    InstallDir := ExtractFileDir(Uninstaller);

  // Without _?= an NSIS uninstaller copies itself to %TEMP% and returns at
  // once, so the wait below would end before it finished (and it could then
  // delete the shortcuts this installer is about to create).
  Exec(Uninstaller, '/S _?=' + InstallDir, '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  // _?= leaves the uninstaller and its folder behind.
  DeleteFile(Uninstaller);
  RemoveDir(InstallDir);
end;
