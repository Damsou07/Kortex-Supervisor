$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$tools = Join-Path $workspace '.test-data\tools'
$root = Join-Path $tools 'inno-7.1.1'
$download = Join-Path $tools 'innosetup-7.1.1-x64.exe'
$expectedHash = '8A5A6711C3766A6E5FB74F280BA0E3551E719CE94538D901FDBD1F8F9920B3F0'
New-Item -ItemType Directory -Path $tools -Force | Out-Null
if (-not (Test-Path -LiteralPath $download)) {
    Invoke-WebRequest -Uri 'https://github.com/jrsoftware/issrc/releases/download/is-7_1_1/innosetup-7.1.1-x64.exe' -OutFile $download
}
if ((Get-FileHash -LiteralPath $download -Algorithm SHA256).Hash -ne $expectedHash) { throw 'Empreinte Inno Setup 7.1.1 incorrecte' }
$signature = Get-AuthenticodeSignature -LiteralPath $download
if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'Pyrsys B.V.') { throw 'Signature Inno Setup non validée' }
if (-not (Test-Path -LiteralPath (Join-Path $root 'ISCC.exe'))) {
    $process = Start-Process -FilePath $download -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CURRENTUSER', '/PORTABLE=1', ('/DIR="' + $root + '"')) -WindowStyle Hidden -PassThru -Wait
    if ($process.ExitCode -ne 0) { throw "Préparation Inno Setup : code $($process.ExitCode)" }
}
Write-Output (Join-Path $root 'ISCC.exe')
