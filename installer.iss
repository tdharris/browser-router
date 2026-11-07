#define MyAppName "Browser Router"
#ifndef MyAppVersion
  #define MyAppVersion "0.0.2"
#endif
#define MyAppExeName "browser-router.exe"

[Setup]
AppId={{E8CB3055-08F9-4C51-8C89-A25F2D1CD5E8}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher=Browser Router
DefaultDirName={localappdata}\Programs\Browser Router
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=dist
OutputBaseFilename=browser-router-{#MyAppVersion}-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
SetupIconFile=assets\app-icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}

[Files]
Source: "target\x86_64-pc-windows-msvc\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "config.toml"; DestDir: "{app}"; DestName: "config.example.toml"; Flags: ignoreversion

[Icons]
Name: "{group}\Edit Browser Router configuration"; Filename: "notepad.exe"; Parameters: """{userappdata}\browser-router\config.toml"""
Name: "{group}\Windows Default Apps"; Filename: "ms-settings:defaultapps?app=BrowserRouter"

[Run]
Filename: "{app}\{#MyAppExeName}"; Parameters: "--register"; Flags: runhidden waituntilterminated
Filename: "ms-settings:defaultapps?app=BrowserRouter"; Description: "Choose Browser Router for HTTP and HTTPS"; Flags: shellexec postinstall skipifsilent unchecked

[UninstallRun]
Filename: "{app}\{#MyAppExeName}"; Parameters: "--unregister"; Flags: runhidden waituntilterminated
