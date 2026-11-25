; Inno Setup script for APK Analysis Toolkit (apkat).
; Installs per-user (no admin needed) and adds the install dir to the
; user's PATH so "apkat" works from any shell after install.
#define MyAppName "APK Analysis Toolkit"
#define MyAppExeName "apkat.exe"
#ifndef MyAppVersion
  #define MyAppVersion "0.0.0"
#endif

[Setup]
AppId={{B6B2B6B4-6B0A-4B7E-9C7B-APKAT00000001}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
DefaultDirName={autopf}\APK Analysis Toolkit
DefaultGroupName=APK Analysis Toolkit
PrivilegesRequired=lowest
OutputBaseFilename=apkat-setup
OutputDir=installer-out
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
DisableProgramGroupPage=yes

[Files]
Source: "target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\APK Analysis Toolkit"; Filename: "{app}\{#MyAppExeName}"

[Code]
const
  EnvKey = 'Environment';
  HWND_BROADCAST = $FFFF;
  WM_SETTINGCHANGE = $1A;
  SMTO_ABORTIFHUNG = 2;

function SendMessageTimeout(hWnd: LongInt; Msg: LongInt; wParam: LongInt;
  lParam: string; fuFlags: LongInt; uTimeout: LongInt; var lpdwResult: LongInt): LongInt;
  external 'SendMessageTimeoutW@user32.dll stdcall';

procedure BroadcastEnvChange();
var
  Dummy: LongInt;
begin
  SendMessageTimeout(HWND_BROADCAST, WM_SETTINGCHANGE, 0, 'Environment', SMTO_ABORTIFHUNG, 1000, Dummy);
end;

procedure AddToPath();
var
  Path: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Path) then
    Path := '';
  if Pos(LowerCase(ExpandConstant('{app}') + ';'), LowerCase(Path + ';')) = 0 then begin
    if (Path <> '') and (Path[Length(Path)] <> ';') then
      Path := Path + ';';
    Path := Path + ExpandConstant('{app}');
    RegWriteStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Path);
    BroadcastEnvChange();
  end;
end;

procedure RemoveFromPath();
var
  Path, Entry: string;
  P: Integer;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Path) then
    exit;
  Entry := ExpandConstant('{app}');
  P := Pos(LowerCase(Entry + ';'), LowerCase(Path + ';'));
  if P > 0 then begin
    Delete(Path, P, Length(Entry) + 1);
    RegWriteStringValue(HKEY_CURRENT_USER, EnvKey, 'Path', Path);
    BroadcastEnvChange();
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    AddToPath();
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath();
end;

[UninstallDelete]
Type: files; Name: "{app}\apkat.old.exe"
Type: files; Name: "{app}\apkat.new.exe"
