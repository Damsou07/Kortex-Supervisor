#requires -Version 7.2
param([string]$DistPath = (Join-Path (Split-Path -Parent $PSScriptRoot) 'dist'))
$ErrorActionPreference = 'Stop'
$payload = Join-Path $DistPath 'Kortex-Supervisor'
$setup = Join-Path $DistPath 'setup.exe'
if (-not (Test-Path -LiteralPath $setup)) { throw 'setup.exe absent' }
$manifest = Get-Content -LiteralPath (Join-Path $payload 'manifest.json') -Raw | ConvertFrom-Json
$version = (Get-Item -LiteralPath $setup).VersionInfo.ProductVersion.Trim()
if ($version -ne $manifest.version) { throw "Version du setup incorrecte : $version" }
foreach ($required in @('kortex-supervisor.exe', 'kortex.ico', 'vcruntime140.dll', 'msvcp140.dll', 'README.md', 'docs/UTILISATION.md', 'licenses/dependencies.txt')) {
    if (-not ($manifest.files.path -contains $required)) { throw "Livraison incomplète : $required" }
}
foreach ($file in $manifest.files) {
    $actual = (Get-FileHash -LiteralPath (Join-Path $payload $file.path) -Algorithm SHA256).Hash
    if ($actual -ne $file.sha256) { throw "Empreinte invalide : $($file.path)" }
}
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead((Join-Path $DistPath 'Kortex-Supervisor-windows-x64.zip'))
try {
    foreach ($file in $manifest.files) {
        $zipEntry = $zip.Entries | Where-Object { $_.FullName.Replace('\', '/') -eq $file.path }
        if (@($zipEntry).Count -ne 1) { throw "Fichier absent ou dupliqué dans le ZIP : $($file.path)" }
        $stream = $zipEntry.Open()
        $sha = [Security.Cryptography.SHA256]::Create()
        try { $actual = [Convert]::ToHexString($sha.ComputeHash($stream)) }
        finally { $stream.Dispose(); $sha.Dispose() }
        if ($actual -ne $file.sha256) { throw "ZIP différent du payload : $($file.path)" }
    }
} finally { $zip.Dispose() }
foreach ($name in @('setup.exe', 'Kortex-Supervisor-windows-x64.zip')) {
    $expected = (Get-Content -LiteralPath (Join-Path $DistPath "$name.sha256") -Raw).Split(' ')[0]
    if ((Get-FileHash -LiteralPath (Join-Path $DistPath $name) -Algorithm SHA256).Hash -ne $expected) { throw "Empreinte invalide : $name" }
}
Write-Output "Livraison vérifiée : version $($manifest.version), $($manifest.files.Count) fichiers, ZIP et setup.exe."
