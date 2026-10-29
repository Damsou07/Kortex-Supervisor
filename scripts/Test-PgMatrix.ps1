param([ValidateSet('16','17')][string[]]$Major = @('16','17'), [switch]$Tls)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$toolsRoot = Join-Path $workspace '.test-data\tools'
New-Item -ItemType Directory -Path $toolsRoot -Force | Out-Null
$versions = @(
    @{Major='16';Version='16.15-5';Port=55436},
    @{Major='17';Version='17.11-5';Port=55437}
)
Add-Type -AssemblyName System.IO.Compression.FileSystem
foreach ($entry in $versions) {
    if ($entry.Major -notin $Major) { continue }
    $destination = Join-Path $toolsRoot ('pg' + $entry.Major)
    $pgBin = Join-Path $destination 'pgsql\bin'
    if (-not (Test-Path -LiteralPath (Join-Path $pgBin 'pg_ctl.exe'))) {
        $archivePath = Join-Path $toolsRoot ('pg' + $entry.Major + '.zip')
        $url = 'https://get.enterprisedb.com/postgresql/postgresql-' + $entry.Version + '-windows-x64-binaries.zip'
        Write-Output ('Téléchargement PostgreSQL ' + $entry.Version)
        & curl.exe --fail --location --silent --show-error --output $archivePath $url
        if ($LASTEXITCODE -ne 0) { throw 'Échec téléchargement PostgreSQL' }
        $archive = [IO.Compression.ZipFile]::OpenRead($archivePath)
        try {
            $rootPath = [IO.Path]::GetFullPath($destination) + [IO.Path]::DirectorySeparatorChar
            foreach ($file in $archive.Entries) {
                if ($file.FullName -notmatch '^pgsql/(bin/|share/|lib/[^/]+\.dll$)' -or $file.Name -eq '') { continue }
                $output = [IO.Path]::GetFullPath((Join-Path $destination $file.FullName))
                if (-not $output.StartsWith($rootPath, [StringComparison]::OrdinalIgnoreCase)) { throw 'Chemin hors archive autorisée' }
                New-Item -ItemType Directory -Path ([IO.Path]::GetDirectoryName($output)) -Force | Out-Null
                [IO.Compression.ZipFileExtensions]::ExtractToFile($file, $output, $true)
            }
        } finally { $archive.Dispose() }
        Remove-Item -LiteralPath $archivePath -Force
    }
    & (Join-Path $PSScriptRoot 'Test-Postgres.ps1') -PgBin $pgBin -Port $entry.Port -Tls:$Tls
}
