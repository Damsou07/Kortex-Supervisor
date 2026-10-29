#Requires -Version 7.2
param([Parameter(Mandatory)][string]$Directory)
$ErrorActionPreference = 'Stop'
# Ephemeral test PKI; no certificate is installed in any Windows store.
$caKey = [Security.Cryptography.RSA]::Create(2048)
$serverKey = [Security.Cryptography.RSA]::Create(2048)
$hash = [Security.Cryptography.HashAlgorithmName]::SHA256
$padding = [Security.Cryptography.RSASignaturePadding]::Pkcs1
$now = [DateTimeOffset]::UtcNow
try {
    $caRequest = [Security.Cryptography.X509Certificates.CertificateRequest]::new('CN=Kortex Disposable Test CA', $caKey, $hash, $padding)
    $caRequest.CertificateExtensions.Add([Security.Cryptography.X509Certificates.X509BasicConstraintsExtension]::new($true, $false, 0, $true))
    $caRequest.CertificateExtensions.Add([Security.Cryptography.X509Certificates.X509KeyUsageExtension]::new([Security.Cryptography.X509Certificates.X509KeyUsageFlags]::KeyCertSign, $true))
    $ca = $caRequest.CreateSelfSigned($now.AddMinutes(-5), $now.AddDays(1))
    $serverRequest = [Security.Cryptography.X509Certificates.CertificateRequest]::new('CN=localhost', $serverKey, $hash, $padding)
    $serverRequest.CertificateExtensions.Add([Security.Cryptography.X509Certificates.X509BasicConstraintsExtension]::new($false, $false, 0, $true))
    $san = [Security.Cryptography.X509Certificates.SubjectAlternativeNameBuilder]::new()
    $san.AddDnsName('localhost')
    $serverRequest.CertificateExtensions.Add($san.Build())
    $usages = [Security.Cryptography.OidCollection]::new()
    [void]$usages.Add([Security.Cryptography.Oid]::new('1.3.6.1.5.5.7.3.1'))
    $serverRequest.CertificateExtensions.Add([Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]::new($usages, $false))
    $serial = [Security.Cryptography.RandomNumberGenerator]::GetBytes(16)
    $server = $serverRequest.Create($ca, $now.AddMinutes(-4), $now.AddHours(12), $serial)
    [IO.File]::WriteAllText((Join-Path $Directory 'ca.pem'), $ca.ExportCertificatePem())
    [IO.File]::WriteAllText((Join-Path $Directory 'server.crt'), $server.ExportCertificatePem())
    [IO.File]::WriteAllText((Join-Path $Directory 'server.key'), $serverKey.ExportPkcs8PrivateKeyPem())
} finally {
    if ($server) { $server.Dispose() }
    if ($ca) { $ca.Dispose() }
    $serverKey.Dispose()
    $caKey.Dispose()
}
