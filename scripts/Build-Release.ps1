$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
Push-Location $workspace
try {
    cargo build --release --locked -p kortex-supervisor
    if ($LASTEXITCODE -ne 0) { throw 'Échec compilation release' }
    $destination = Join-Path $workspace 'dist\Kortex-Supervisor'
    New-Item -ItemType Directory -Path $destination -Force | Out-Null
    Copy-Item -LiteralPath 'target\release\kortex-supervisor.exe' -Destination $destination -Force
    Copy-Item -LiteralPath 'README.md' -Destination $destination -Force
    Copy-Item -LiteralPath 'docs' -Destination $destination -Recurse -Force
    $fontLicenses = Join-Path $destination 'licenses'
    New-Item -ItemType Directory -Path $fontLicenses -Force | Out-Null
    Copy-Item -Path 'assets\fonts\OFL*.txt' -Destination $fontLicenses -Force
    # Bundle the Microsoft redistributable runtime alongside the portable executable.
    $vswhere = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
    $vsRoot = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if (-not $vsRoot) { throw 'Build Tools C++ stables requis pour le paquet portable' }
    $redistRoot = Join-Path $vsRoot 'VC\Redist\MSVC'
    $redistVersion = Get-ChildItem -LiteralPath $redistRoot -Directory | Where-Object Name -Match '^\d+\.' | Sort-Object Name -Descending | Select-Object -First 1
    $runtime = Get-ChildItem -LiteralPath (Join-Path $redistVersion.FullName 'x64') -Directory | Where-Object Name -Like 'Microsoft.VC*.CRT' | Select-Object -First 1
    if (-not $runtime) { throw 'Runtime Visual C++ redistribuable introuvable' }
    Copy-Item -Path (Join-Path $runtime.FullName '*.dll') -Destination $destination -Force
    $runtimeNotice = Join-Path $vsRoot 'Licenses\1036\Redist.txt'
    if (Test-Path -LiteralPath $runtimeNotice) { Copy-Item -LiteralPath $runtimeNotice -Destination (Join-Path $fontLicenses 'Microsoft-Redist.txt') -Force }
    $metadata = cargo metadata --locked --format-version 1 | ConvertFrom-Json
    if ($LASTEXITCODE -ne 0) { throw 'Impossible de produire les mentions des dépendances' }
    $notices = [Text.StringBuilder]::new()
    foreach ($package in $metadata.packages | Where-Object source | Sort-Object name) {
        [void]$notices.AppendLine(($package.name + ' ' + $package.version + ' — ' + $package.license))
        $packageDir = Split-Path -Parent $package.manifest_path
        foreach ($license in Get-ChildItem -LiteralPath $packageDir -File | Where-Object Name -Match '^(LICENSE|COPYING|NOTICE)') {
            [void]$notices.AppendLine([IO.File]::ReadAllText($license.FullName))
        }
        [void]$notices.AppendLine("`n----------------------------------------`n")
    }
    [IO.File]::WriteAllText((Join-Path $fontLicenses 'dependencies.txt'), $notices.ToString())
    Compress-Archive -Path (Join-Path $destination '*') -DestinationPath (Join-Path $workspace 'dist\Kortex-Supervisor-windows-x64.zip') -Force
    $archiveHash = Get-FileHash -LiteralPath (Join-Path $workspace 'dist\Kortex-Supervisor-windows-x64.zip') -Algorithm SHA256
    [IO.File]::WriteAllText((Join-Path $workspace 'dist\Kortex-Supervisor-windows-x64.zip.sha256'), ($archiveHash.Hash.ToLowerInvariant() + '  Kortex-Supervisor-windows-x64.zip' + "`n"))
    $archiveHash
} finally { Pop-Location }
