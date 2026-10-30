param([Parameter(Mandatory)][string]$OutputDirectory)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
New-Item -ItemType Directory -Path $OutputDirectory -Force | Out-Null
$logo = [Drawing.Image]::FromFile((Join-Path (Split-Path -Parent $PSScriptRoot) 'logo-Kortex.png'))
try {
    # PNG-compressed ICO frames retain alpha without native handles or external tools.
    $frames = @()
    foreach ($size in @(16, 32, 48, 64, 128, 256)) {
        $bitmap = [Drawing.Bitmap]::new($size, $size)
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        $stream = [IO.MemoryStream]::new()
        try {
            $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $graphics.DrawImage($logo, 0, 0, $size, $size)
            $bitmap.Save($stream, [Drawing.Imaging.ImageFormat]::Png)
            $frames += @{ Size = $size; Data = $stream.ToArray() }
        } finally { $stream.Dispose(); $graphics.Dispose(); $bitmap.Dispose() }
    }
    $writer = [IO.BinaryWriter]::new([IO.File]::Create((Join-Path $OutputDirectory 'kortex.ico')))
    try {
        $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$frames.Count)
        $offset = 6 + 16 * $frames.Count
        foreach ($frame in $frames) {
            $dimension = [byte]($frame.Size % 256)
            $writer.Write($dimension); $writer.Write($dimension)
            $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]32)
            $writer.Write([uint32]$frame.Data.Length); $writer.Write([uint32]$offset)
            $offset += $frame.Data.Length
        }
        foreach ($frame in $frames) { $writer.Write([byte[]]$frame.Data) }
    } finally { $writer.Dispose() }
    foreach ($spec in @(@{Name='wizard'; Width=328; Height=628}, @{Name='wizard-small'; Width=128; Height=128})) {
        $bitmap = [Drawing.Bitmap]::new($spec.Width, $spec.Height)
        $graphics = [Drawing.Graphics]::FromImage($bitmap)
        $gold = [Drawing.SolidBrush]::new([Drawing.Color]::FromArgb(255, 200, 48))
        $red = [Drawing.SolidBrush]::new([Drawing.Color]::FromArgb(255, 57, 66))
        $font = [Drawing.Font]::new('Segoe UI', 24, [Drawing.FontStyle]::Bold, [Drawing.GraphicsUnit]::Pixel)
        $smallFont = [Drawing.Font]::new('Segoe UI', 16, [Drawing.FontStyle]::Regular, [Drawing.GraphicsUnit]::Pixel)
        try {
            $graphics.Clear([Drawing.Color]::FromArgb(17, 19, 24))
            $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            if ($spec.Name -eq 'wizard') {
                $graphics.FillRectangle($red, 0, 0, 5, $spec.Height)
                $graphics.DrawImage($logo, 24, 54, 280, 280)
                $graphics.DrawString('KORTEX', $font, $gold, 28, 380)
                $graphics.DrawString('SUPERVISOR', $smallFont, [Drawing.Brushes]::White, 30, 416)
                $graphics.FillRectangle($red, 30, 464, 90, 3)
                $graphics.DrawString("Vos données.`nVotre espace.", $smallFont, [Drawing.Brushes]::LightGray, 30, 500)
            } else { $graphics.DrawImage($logo, 0, 0, 128, 128) }
            $bitmap.Save((Join-Path $OutputDirectory ($spec.Name + '.bmp')), [Drawing.Imaging.ImageFormat]::Bmp)
        } finally { $smallFont.Dispose(); $font.Dispose(); $red.Dispose(); $gold.Dispose(); $graphics.Dispose(); $bitmap.Dispose() }
    }
} finally { $logo.Dispose() }
