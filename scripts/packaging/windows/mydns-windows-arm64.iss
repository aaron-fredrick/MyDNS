#ifndef MyDNSVersion
#define MyDNSVersion "1.0.0"
#endif

[Setup]
AppId={{8A1E3E53-9F0A-4D1A-9A72-6E1C6E6D2B01}
AppName=MyDNS
AppVersion={#MyDNSVersion}
VersionInfoVersion={#MyDNSVersion}
VersionInfoDescription=MyDNS local DNS resolver and management dashboard
VersionInfoProductName=MyDNS
VersionInfoCompany=MyDNS
AppPublisher=MyDNS
AppPublisherURL=https://github.com/aaron-fredrick/MyDNS
AppSupportURL=https://github.com/aaron-fredrick/MyDNS/issues
DefaultDirName={autopf}\MyDNS
DefaultGroupName=MyDNS
DisableProgramGroupPage=yes
DisableDirPage=no
OutputDir=..\..\..\out\installers
OutputBaseFilename=MyDNS-v{#MyDNSVersion}-windows-arm64-setup
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
PrivilegesRequired=admin
UninstallDisplayName=MyDNS
WizardStyle=modern
Compression=lzma2
SolidCompression=yes

[Files]
Source: "..\..\..\out\win-arm64\bin\mydns.exe"; DestDir: "{app}\bin"; Flags: ignoreversion
Source: "..\..\..\out\web\*"; DestDir: "{app}\web"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\..\..\config.toml.example"; DestDir: "{commonappdata}\MyDNS\config"; DestName: "mydns.toml"; Flags: onlyifdoesntexist

[Dirs]
Name: "{commonappdata}\MyDNS\config"
Name: "{commonappdata}\MyDNS\data"
Name: "{commonappdata}\MyDNS\logs"

[UninstallDelete]
; Application binaries/assets are removed; persistent user state is intentionally retained.
Type: filesandordirs; Name: "{app}"
