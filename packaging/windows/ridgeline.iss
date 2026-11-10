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
PrivilegesRequiredOverridesAllowed=dialog
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
