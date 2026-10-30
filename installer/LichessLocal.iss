#ifndef SourceDir
 #define SourceDir "..\artifacts\publish"
#endif
#ifndef RuntimeInstaller
 #define RuntimeInstaller "..\artifacts\downloads\MicrosoftEdgeWebView2RuntimeInstallerX64.exe"
#endif
[Setup]
AppId={{1038AECE-952C-47CC-9B6C-C317A3584656}
AppName=Lichess Local
AppVersion=0.1.0
AppPublisher=Ian Rastall
AppVerName=Lichess Local 0.1.0
DefaultDirName={localappdata}\Programs\Jupiter\LichessLocal
DefaultGroupName=Lichess Local
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.26100
UninstallDisplayIcon={app}\LichessLocal.exe
SetupIconFile=..\app\Assets\AppIcon.ico
OutputDir=..\artifacts\installer
OutputBaseFilename=LichessLocal-0.1.0-Setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
AppMutex=Jupiter.LichessLocal
[Files]
Source: "{#SourceDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#RuntimeInstaller}"; DestDir: "{tmp}"; Flags: deleteafterinstall; Check: NeedsWebView2
[Icons]
Name: "{autoprograms}\Lichess Local"; Filename: "{app}\LichessLocal.exe"; AppUserModelID: "Jupiter.LichessLocal"
[Run]
Filename: "{app}\LichessLocal.exe"; Description: "Launch Lichess Local"; Flags: nowait postinstall skipifsilent
[Code]
function NeedsWebView2: Boolean;
var Version: string;
begin
 Result := not ((RegQueryStringValue(HKLM32, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Version) or RegQueryStringValue(HKCU, 'SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}', 'pv', Version)) and (Version <> '') and (Version <> '0.0.0.0'));
end;
procedure CurStepChanged(CurStep: TSetupStep);
var ResultCode: Integer;
begin
 if (CurStep = ssPostInstall) and NeedsWebView2 then
 begin
  if not Exec(ExpandConstant('{tmp}\MicrosoftEdgeWebView2RuntimeInstallerX64.exe'), '/silent /install', '', SW_HIDE, ewWaitUntilTerminated, ResultCode) then
   RaiseException('The bundled WebView2 runtime could not be started.');
  if NeedsWebView2 then
   RaiseException('The WebView2 runtime could not be installed. Re-run setup after resolving the runtime installation error.');
 end;
end;
