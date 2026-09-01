#define MyDNSVersion "1.0.0"

[Setup]
AppName=MyDNS
AppVersion={#MyDNSVersion}
DefaultDirName={autopf}\MyDNS
DefaultGroupName=MyDNS
OutputBaseFilename=MyDNS-v{#MyDNSVersion}-windows-x64-setup
ArchitecturesInstallIn64BitMode=x64
PrivilegesRequired=admin
UninstallDisplayName=MyDNS

[Files]
Source: "..\..\out\win-x64\bin\mydns.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "..\..\out\web\*"; DestDir: "{app}\web"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\..\config.toml.example"; DestDir: "{commonappdata}\MyDNS\config"; DestName: "mydns.toml"; Flags: onlyifdoesntexist

[Dirs]
Name: "{commonappdata}\MyDNS\config"
Name: "{commonappdata}\MyDNS\data"
Name: "{commonappdata}\MyDNS\logs"

[UninstallDelete]
Type: filesandordirs; Name: "{app}"
