#requires -Version 7.2
param([switch]$DisposableEnvironment)
$ErrorActionPreference = 'Stop'
if (-not $DisposableEnvironment) { throw 'Ce test installe réellement le produit. Exécuter uniquement dans une VM Windows jetable avec -DisposableEnvironment.' }
$workspace = Split-Path -Parent $PSScriptRoot
$setup = Join-Path $workspace 'dist\setup.exe'
$manifest = Get-Content -LiteralPath (Join-Path $workspace 'dist\Kortex-Supervisor\manifest.json') -Raw | ConvertFrom-Json
$identity = '{52CC140A-90B6-4D62-A987-68AFDAB888EA}_is1'
$runRoot = Join-Path $workspace ('target\installer-smoke\' + [guid]::NewGuid().ToString('N'))

function Invoke-SetupProcess([string]$Executable, [string[]]$Options) {
    $process = Start-Process -FilePath $Executable -ArgumentList $Options -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw "Échec du processus d’installation : $($process.ExitCode)" }
}
function Assert-InstalledFiles([string]$Directory) {
    foreach ($file in $manifest.files) {
        if ((Get-FileHash -LiteralPath (Join-Path $Directory $file.path) -Algorithm SHA256).Hash -ne $file.sha256) { throw "Fichier installé incorrect : $($file.path)" }
    }
}

foreach ($mode in @('CURRENTUSER', 'ALLUSERS')) {
    $registry = if ($mode -eq 'CURRENTUSER') { 'HKCU:' } else { 'HKLM:' }
    $uninstallKey = "$registry\Software\Microsoft\Windows\CurrentVersion\Uninstall\$identity"
    $desktop = [Environment]::GetFolderPath($(if ($mode -eq 'CURRENTUSER') { 'DesktopDirectory' } else { 'CommonDesktopDirectory' }))
    $programs = [Environment]::GetFolderPath($(if ($mode -eq 'CURRENTUSER') { 'Programs' } else { 'CommonPrograms' }))
    $shortcuts = @((Join-Path $desktop 'Kortex Supervisor.lnk'), (Join-Path $programs 'Kortex Supervisor.lnk'))
    if ((Test-Path -LiteralPath $uninstallKey) -or @($shortcuts | Where-Object { Test-Path -LiteralPath $_ }).Count) { throw 'Installation ou raccourci préexistant : test interrompu pour le préserver.' }
    $root = Join-Path $runRoot $mode
    $appDir = Join-Path $root 'Application avec espaces'
    $config = Join-Path $root 'configuration\workspace.json'
    New-Item -ItemType Directory -Path (Split-Path -Parent $config) -Force | Out-Null
    [IO.File]::WriteAllText($config, '{"version":1,"data_refresh_secs":7,"profiles":[],"diagrams":{}}')
    $database = Join-Path $root 'données.db'
    [IO.File]::WriteAllBytes($database, [byte[]](0..255))
    $configHash = (Get-FileHash -LiteralPath $config).Hash
    $databaseHash = (Get-FileHash -LiteralPath $database).Hash
    $options = @("/$mode", '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/TASKS=desktopicon')
    Invoke-SetupProcess $setup ($options + @(('/DIR="' + $appDir + '"'), ('/LOG="' + (Join-Path $root 'install.log') + '"')))
    Assert-InstalledFiles $appDir
    $registration = Get-ItemProperty -LiteralPath $uninstallKey
    if ($registration.DisplayVersion -ne $manifest.version) { throw 'Version installée incorrecte' }
    foreach ($shortcut in $shortcuts) {
        if (-not (Test-Path -LiteralPath $shortcut)) { throw "Raccourci absent : $shortcut" }
        $shell = New-Object -ComObject WScript.Shell
        $link = $shell.CreateShortcut($shortcut)
        if ($link.TargetPath -ne (Join-Path $appDir 'kortex-supervisor.exe') -or $link.IconLocation -notlike "$appDir\kortex.ico*") { throw 'Cible ou icône du raccourci incorrecte' }
    }
    # No /DIR on reinstall: the stable AppId must recover the previous path.
    Invoke-SetupProcess $setup ($options + @('/LOG="' + (Join-Path $root 'reinstall.log') + '"'))
    Assert-InstalledFiles $appDir
    $sentinel = Join-Path $appDir 'fichier-utilisateur.txt'
    [IO.File]::WriteAllText($sentinel, 'À conserver')
    $uninstaller = @(Get-ChildItem -LiteralPath $appDir -Filter 'unins*.exe')
    if ($uninstaller.Count -ne 1) { throw 'Désinstalleur absent ou ambigu' }
    Invoke-SetupProcess $uninstaller[0].FullName @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', ('/LOG="' + (Join-Path $root 'uninstall.log') + '"'))
    if ((Test-Path -LiteralPath (Join-Path $appDir 'kortex-supervisor.exe')) -or (Test-Path -LiteralPath $uninstallKey)) { throw 'Désinstallation incomplète' }
    foreach ($shortcut in $shortcuts) { if (Test-Path -LiteralPath $shortcut) { throw 'Raccourci non retiré' } }
    if ((Get-FileHash -LiteralPath $config).Hash -ne $configHash -or (Get-FileHash -LiteralPath $database).Hash -ne $databaseHash -or -not (Test-Path -LiteralPath $sentinel)) { throw 'Données utilisateur altérées' }
    Write-Output "$mode : installation, réinstallation, raccourcis et désinstallation réussis ; données conservées."
}
