param([string]$PgBin = 'C:\Program Files\PostgreSQL\18\bin', [int]$Port = 55432, [switch]$Tls)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$dataParent = Join-Path $workspace '.test-data'
New-Item -ItemType Directory -Path $dataParent -Force | Out-Null
$data = Join-Path $dataParent ('pg-' + [guid]::NewGuid().ToString('N'))
$initdb = Join-Path $PgBin 'initdb.exe'
$pgctl = Join-Path $PgBin 'pg_ctl.exe'
if (-not (Test-Path -LiteralPath $initdb)) { throw "PostgreSQL introuvable : $PgBin" }
& $initdb -D $data -A trust -U kortex --encoding=UTF8 --no-locale
if ($LASTEXITCODE -ne 0) { throw 'Échec initdb' }
$started = $false
try {
    $serverOptions = "-p $Port -h 127.0.0.1"
    if ($Tls) {
        & (Join-Path $PSScriptRoot 'New-TestCertificates.ps1') -Directory $data
        $serverOptions += ' -c ssl=on'
        $env:KORTEX_TEST_CA = Join-Path $data 'ca.pem'
    }
    & $pgctl -D $data -l (Join-Path $dataParent 'postgres-test.log') -o $serverOptions -w start
    if ($LASTEXITCODE -ne 0) { throw 'Échec démarrage PostgreSQL' }
    $started = $true
    $env:KORTEX_TEST_PG_PORT = "$Port"
    Push-Location $workspace
    try {
        $testTarget = if ($Tls) { 'postgres_tls' } else { 'postgres' }
        cargo test -p kortex-infrastructure --test $testTarget --locked -- --ignored --nocapture
        if ($LASTEXITCODE -ne 0) { throw 'Échec tests PostgreSQL' }
    } finally { Pop-Location }
} finally {
    if ($started) { & $pgctl -D $data -m fast -w stop }
    Remove-Item Env:KORTEX_TEST_PG_PORT -ErrorAction SilentlyContinue
    Remove-Item Env:KORTEX_TEST_CA -ErrorAction SilentlyContinue
    $resolvedData = [IO.Path]::GetFullPath($data)
    $resolvedParent = [IO.Path]::GetFullPath($dataParent) + [IO.Path]::DirectorySeparatorChar
    if (-not $resolvedData.StartsWith($resolvedParent, [StringComparison]::OrdinalIgnoreCase)) { throw 'Chemin de nettoyage hors dossier de test' }
    if (Test-Path -LiteralPath $resolvedData) { Remove-Item -LiteralPath $resolvedData -Recurse -Force }
}
