#ifndef AppVersion
  #error AppVersion requis (version Cargo)
#endif
#ifndef PayloadDir
  #error PayloadDir requis
#endif
#ifndef AssetsDir
  #error AssetsDir requis
#endif

[Setup]
AppId={{52CC140A-90B6-4D62-A987-68AFDAB888EA}
AppName=Kortex Supervisor
AppVersion={#AppVersion}
AppPublisher=Kortex
VersionInfoVersion={#AppVersion}.0
VersionInfoProductVersion={#AppVersion}
DefaultDirName={autopf}\Kortex Supervisor
DefaultGroupName=Kortex Supervisor
DisableProgramGroupPage=yes
DisableDirPage=no
DisableWelcomePage=no
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
UsePreviousAppDir=yes
UsePreviousPrivileges=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0.17763
OutputBaseFilename=setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern dark includetitlebar hidebevels
WizardBackColor=#111318
WizardImageBackColor=#111318
WizardSmallImageBackColor=#111318
WizardImageFile={#AssetsDir}\wizard.bmp
WizardSmallImageFile={#AssetsDir}\wizard-small.bmp
SetupIconFile={#AssetsDir}\kortex.ico
UninstallDisplayIcon={app}\kortex.ico
CloseApplications=yes
RestartApplications=no

[Languages]
Name: french; MessagesFile: compiler:Languages\French.isl

[Tasks]
Name: desktopicon; Description: "Créer une icône sur le bureau"; GroupDescription: "Raccourcis :"

[Files]
Source: "{#PayloadDir}\kortex-supervisor.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\*.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\kortex.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\manifest.json"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PayloadDir}\docs\*"; DestDir: "{app}\docs"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#PayloadDir}\licenses\*"; DestDir: "{app}\licenses"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Kortex Supervisor"; Filename: "{app}\kortex-supervisor.exe"; WorkingDir: "{app}"; IconFilename: "{app}\kortex.ico"
Name: "{autodesktop}\Kortex Supervisor"; Filename: "{app}\kortex-supervisor.exe"; WorkingDir: "{app}"; IconFilename: "{app}\kortex.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\kortex-supervisor.exe"; Description: "Lancer Kortex Supervisor"; Flags: nowait postinstall skipifsilent unchecked runasoriginaluser

[Messages]
WelcomeLabel2=Installez Kortex Supervisor pour explorer PostgreSQL et SQLite dans votre espace de travail local.%n%nVos bases de données et préférences existantes sont conservées.
FinishedLabel=Kortex Supervisor est installé.%n%nConsultez Paramètres → Chemins locaux pour retrouver les emplacements utilisés par l’application.
