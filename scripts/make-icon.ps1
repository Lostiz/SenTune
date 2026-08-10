Add-Type -AssemblyName System.Drawing

$root = Split-Path -Parent $PSScriptRoot
$out = Join-Path $root "src-tauri\icons"
New-Item -ItemType Directory -Force -Path $out | Out-Null

function New-RoundedPath([System.Drawing.Rectangle]$rect, [int]$radius) {
  $path = New-Object System.Drawing.Drawing2D.GraphicsPath
  $d = $radius * 2
  $path.AddArc($rect.X, $rect.Y, $d, $d, 180, 90)
  $path.AddArc($rect.X + $rect.Width - $d, $rect.Y, $d, $d, 270, 90)
  $path.AddArc($rect.X + $rect.Width - $d, $rect.Y + $rect.Height - $d, $d, $d, 0, 90)
  $path.AddArc($rect.X, $rect.Y + $rect.Height - $d, $d, $d, 90, 90)
  $path.CloseFigure()
  return $path
}

function New-AppBitmap([int]$size) {
  $bmp = New-Object System.Drawing.Bitmap($size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb))
  $g = [System.Drawing.Graphics]::FromImage($bmp)
  $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
  $g.Clear([System.Drawing.Color]::Transparent)

  # Dark rounded base
  $bgRect = [System.Drawing.Rectangle]::new(0, 0, $size, $size)
  $bgPath = New-RoundedPath $bgRect ([int]($size * 0.22))
  $bgBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 14, 14, 18))
  $g.FillPath($bgBrush, $bgPath)

  # Red gradient rounded shape
  $inset = [int]($size * 0.16)
  $innerRect = [System.Drawing.Rectangle]::new($inset, $inset, ($size - 2 * $inset), ($size - 2 * $inset))
  $innerPath = New-RoundedPath $innerRect ([int]($innerRect.Width * 0.30))
  $gradient = [System.Drawing.Drawing2D.LinearGradientBrush]::new(
    $innerRect,
    ([System.Drawing.Color]::FromArgb(255, 255, 90, 110)),
    ([System.Drawing.Color]::FromArgb(255, 224, 36, 64)),
    45.0
  )
  $g.FillPath($gradient, $innerPath)

  # White play triangle
  $cx = $size / 2.0 + $size * 0.04
  $cy = $size / 2.0
  $r = $size * 0.17
  $points = [System.Drawing.PointF[]]@(
    [System.Drawing.PointF]::new($cx - $r * 0.45, $cy - $r),
    [System.Drawing.PointF]::new($cx - $r * 0.45, $cy + $r),
    [System.Drawing.PointF]::new($cx + $r, $cy)
  )
  $white = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::White)
  $g.FillPolygon($white, $points)

  $g.Dispose()
  return $bmp
}

$pngPaths = @()
foreach ($size in @(256, 128, 48, 32, 16)) {
  $bmp = New-AppBitmap $size
  $path = Join-Path $out "app-$size.png"
  $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
  $bmp.Dispose()
  $pngPaths += $path
}

# Name mapping used by tauri.conf.json
Copy-Item (Join-Path $out "app-256.png") (Join-Path $out "128x128@2x.png") -Force
Copy-Item (Join-Path $out "app-128.png") (Join-Path $out "128x128.png") -Force
Copy-Item (Join-Path $out "app-32.png") (Join-Path $out "32x32.png") -Force

# Multi-size PNG-compressed ICO (16/32/48/128/256)
$images = @()
$sizes = @()
foreach ($size in @(16, 32, 48, 128, 256)) {
  $images += ,([System.IO.File]::ReadAllBytes((Join-Path $out "app-$size.png")))
  $sizes += $size
}

$ms = New-Object System.IO.MemoryStream
$bw = New-Object System.IO.BinaryWriter($ms)
$count = $images.Count
$bw.Write([UInt16]0)
$bw.Write([UInt16]1)
$bw.Write([UInt16]$count)
$offset = 6 + 16 * $count
for ($i = 0; $i -lt $count; $i++) {
  $dim = $sizes[$i]
  if ($dim -ge 256) { $dim = 0 }
  $bw.Write([Byte]$dim)
  $bw.Write([Byte]$dim)
  $bw.Write([Byte]0)
  $bw.Write([Byte]0)
  $bw.Write([UInt16]1)
  $bw.Write([UInt16]32)
  $bw.Write([UInt32]$images[$i].Length)
  $bw.Write([UInt32]$offset)
  $offset += $images[$i].Length
}
foreach ($image in $images) {
  $bw.Write($image)
}
$bw.Flush()
[System.IO.File]::WriteAllBytes((Join-Path $out "icon.ico"), $ms.ToArray())
$bw.Dispose()
$ms.Dispose()

Write-Output "Icons generated in $out"
