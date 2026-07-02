; T.C.K — TalentCloud Keyboard: Windows Installer (Inno Setup)
; =============================================================
; To build:
;   1. Install Inno Setup (https://jrsoftware.org/isdl.php)
;   2. Run: .\scripts\build.ps1
;   3. In Inno Setup IDE: File > Open > installer/windows/setup.iss > Build
;   4. Or CLI: ISCC.exe installer/windows/setup.iss

#define MyAppName "T.C.K — TalentCloud Keyboard"
#define MyAppVersion "3.0.0"
#define MyAppPublisher "TalentCloud"
#define MyAppURL "https://github.com/Raghuramcoding/keyboard"
#define MyAppExeName "tck-launcher.ps1"

[Setup]
AppId={{4E8B9F2A-3C5D-4A7B-9F1E-2B3C4D5E6F7A}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
DefaultDirName={autopf}\TCK
DefaultGroupName="T.C.K"
DisableProgramGroupPage=yes
OutputDir=..\..\dist
OutputBaseFilename=TCK-Setup-{#MyAppVersion}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
; The server binary.
Source: "..\..\target\release\tck-server.exe"; DestDir: "{app}"; Flags: ignoreversion
; The WASM bundle (served by tck-server).
Source: "..\..\crates\tck-ui\dist\*"; DestDir: "{app}\dist"; Flags: ignoreversion recursesubdirs createallsubdirs
; The launcher script.
Source: "tck-launcher.ps1"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\T.C.K"; Filename: "powershell.exe"; Parameters: "-NoExit -ExecutionPolicy Bypass -File ""{app}\tck-launcher.ps1"""; WorkingDir: "{app}"; Comment: "Launch T.C.K — TalentCloud Keyboard"
Name: "{group}\Uninstall T.C.K"; Filename: "{uninstallexe}"
Name: "{commondesktop}\T.C.K"; Filename: "powershell.exe"; Parameters: "-NoExit -ExecutionPolicy Bypass -File ""{app}\tck-launcher.ps1"""; WorkingDir: "{app}"; Comment: "Launch T.C.K — TalentCloud Keyboard"

[Run]
Filename: "powershell.exe"; Parameters: "-NoExit -ExecutionPolicy Bypass -File ""{app}\tck-launcher.ps1"""; WorkingDir: "{app}"; Description: "Launch T.C.K"; Flags: postinstall nowait skipifsilent shellexec
