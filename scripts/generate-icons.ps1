# Draws every icon Quboid ships from one description, so the executable, the
# tray and the Store package cannot drift apart. Re-run it after changing the
# palette or the mark.
param(
    [string]$AssetDirectory = (Join-Path (Split-Path -Parent $PSScriptRoot) "packaging\msix\assets"),
    [string]$IconPath = (Join-Path (Split-Path -Parent $PSScriptRoot) "crates\quboid-app\quboid.ico"),
    [string]$PreviewPath = "",
    [string]$StoreLogoDirectory = ""
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

# The palette the tray icon and the interface already use.
$Deep = [Drawing.Color]::FromArgb(255, 58, 85, 192)
$Bright = [Drawing.Color]::FromArgb(255, 91, 123, 240)
$Light = [Drawing.Color]::FromArgb(255, 238, 242, 255)

function New-RoundedPath {
    param([float]$X, [float]$Y, [float]$Width, [float]$Height, [float]$Radius)

    $path = New-Object Drawing.Drawing2D.GraphicsPath
    $diameter = $Radius * 2
    if ($diameter -le 0) {
        $path.AddRectangle((New-Object Drawing.RectangleF $X, $Y, $Width, $Height))
        return $path
    }
    $path.AddArc($X, $Y, $diameter, $diameter, 180, 90)
    $path.AddArc($X + $Width - $diameter, $Y, $diameter, $diameter, 270, 90)
    $path.AddArc($X + $Width - $diameter, $Y + $Height - $diameter, $diameter, $diameter, 0, 90)
    $path.AddArc($X, $Y + $Height - $diameter, $diameter, $diameter, 90, 90)
    $path.CloseFigure()
    return $path
}

<#
    Draws the mark: a rounded blue plate carrying four panes, the top-left one
    solid to read as the window being placed. `Plated` draws the plate; the
    unplated variants Windows asks for omit it and draw the panes in one
    colour, because the shell supplies its own background there.
#>
function New-IconBitmap {
    param(
        [int]$Size,
        [int]$Width = 0,
        [bool]$Plated = $true,
        [Drawing.Color]$UnplatedColor = [Drawing.Color]::White
    )

    if ($Width -le 0) { $Width = $Size }
    $bitmap = New-Object Drawing.Bitmap $Width, $Size
    $g = [Drawing.Graphics]::FromImage($bitmap)
    $g.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([Drawing.Color]::Transparent)

    # The mark stays square even on the wide tile, centred in the canvas.
    $extent = [Math]::Min($Width, $Size)
    $offsetX = ($Width - $extent) / 2.0
    $offsetY = ($Size - $extent) / 2.0

    if ($Plated) {
        $margin = $extent * 0.06
        $plate = $extent - 2 * $margin
        $platePath = New-RoundedPath ($offsetX + $margin) ($offsetY + $margin) $plate $plate ($plate * 0.22)
        $brush = New-Object Drawing.Drawing2D.LinearGradientBrush(
            (New-Object Drawing.RectangleF ($offsetX + $margin), ($offsetY + $margin), $plate, $plate),
            $Bright, $Deep, 60.0)
        $g.FillPath($brush, $platePath)
        $brush.Dispose()
        $platePath.Dispose()
        $paneColor = $Light
        $inset = $extent * 0.22
    }
    else {
        $paneColor = $UnplatedColor
        $inset = $extent * 0.16
    }

    # Four panes with a gap between them; the gap and corner radius shrink with
    # the icon so the mark stays legible at 16 pixels.
    $field = $extent - 2 * $inset
    $gap = [Math]::Max($extent * 0.055, 1.0)
    $pane = ($field - $gap) / 2.0
    $radius = [Math]::Max($pane * 0.18, 0.5)
    $originX = $offsetX + $inset
    $originY = $offsetY + $inset

    $positions = @(
        @{ X = 0; Y = 0; Alpha = 255 },
        @{ X = 1; Y = 0; Alpha = 110 },
        @{ X = 0; Y = 1; Alpha = 110 },
        @{ X = 1; Y = 1; Alpha = 110 }
    )
    foreach ($position in $positions) {
        $x = $originX + $position.X * ($pane + $gap)
        $y = $originY + $position.Y * ($pane + $gap)
        $panePath = New-RoundedPath $x $y $pane $pane $radius
        $color = [Drawing.Color]::FromArgb($position.Alpha, $paneColor.R, $paneColor.G, $paneColor.B)
        $paneBrush = New-Object Drawing.SolidBrush $color
        $g.FillPath($paneBrush, $panePath)
        $paneBrush.Dispose()
        $panePath.Dispose()
    }

    $g.Dispose()
    return $bitmap
}

function Save-Icon {
    param([int]$Size, [int]$Width, [bool]$Plated, [Drawing.Color]$UnplatedColor, [string]$Path)

    $bitmap = New-IconBitmap -Size $Size -Width $Width -Plated $Plated -UnplatedColor $UnplatedColor
    $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
}

if ($StoreLogoDirectory) {
    # Artwork for the Store listing. These are upload artefacts rather than
    # build inputs, so they are written wherever the caller asks and nothing is
    # checked in. Drawing them here keeps them from drifting from the app icon.
    if (-not (Test-Path -LiteralPath $StoreLogoDirectory)) {
        New-Item -ItemType Directory -Path $StoreLogoDirectory -Force | Out-Null
    }

    # The tile icons Windows 10/11 customers see. Transparent, like the icon.
    foreach ($size in 300, 150, 71) {
        Save-Icon -Size $size -Width 0 -Plated $true -UnplatedColor $Light `
            -Path (Join-Path $StoreLogoDirectory "StoreTile-${size}x${size}.png")
    }

    <#
        Poster and box art are full-bleed images, so they need a background of
        their own. The mark sits in the middle of the top two thirds because
        the Store may lay text over the bottom third.
    #>
    function Save-PromoArt {
        param([int]$Width, [int]$Height, [string]$Path)

        $bitmap = New-Object Drawing.Bitmap $Width, $Height
        $g = [Drawing.Graphics]::FromImage($bitmap)
        $g.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $g.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $background = New-Object Drawing.Drawing2D.LinearGradientBrush(
            (New-Object Drawing.RectangleF 0, 0, $Width, $Height),
            [Drawing.Color]::White, $Light, 60.0)
        $g.FillRectangle($background, 0, 0, $Width, $Height)
        $background.Dispose()

        $mark = [int]([Math]::Min($Width, $Height) * 0.55)
        $source = New-IconBitmap -Size $mark
        $x = [int](($Width - $mark) / 2)
        $y = [int]($Height * 2.0 / 3.0 / 2.0 - $mark / 2)
        $g.DrawImage($source, $x, $y, $mark, $mark)
        $source.Dispose()
        $g.Dispose()
        $bitmap.Save($Path, [Drawing.Imaging.ImageFormat]::Png)
        $bitmap.Dispose()
    }

    # Partner Center labels the poster art 9:16, but the sizes it accepts are
    # 2:3. The sizes are what matter.
    Save-PromoArt -Width 1440 -Height 2160 -Path (Join-Path $StoreLogoDirectory "PosterArt-1440x2160.png")
    Save-PromoArt -Width 2160 -Height 2160 -Path (Join-Path $StoreLogoDirectory "BoxArt-2160x2160.png")
    Save-PromoArt -Width 1920 -Height 1080 -Path (Join-Path $StoreLogoDirectory "Promotional-1920x1080.png")

    Get-ChildItem -LiteralPath $StoreLogoDirectory -Filter *.png |
        ForEach-Object { Write-Output "Wrote $($_.Name)" }
    return
}

if ($PreviewPath) {
    # One strip showing the sizes that decide whether the mark works, drawn
    # twice: on a light surface and on the dark one the taskbar uses.
    $sizes = @(256, 96, 48, 32, 24, 16)
    $padding = 32
    $band = 320
    $width = [int](($sizes | Measure-Object -Sum).Sum + $padding * ($sizes.Count + 1))
    $preview = New-Object Drawing.Bitmap $width, ([int]($band * 2))
    $g = [Drawing.Graphics]::FromImage($preview)
    $g.Clear([Drawing.Color]::FromArgb(255, 245, 246, 250))
    $dark = New-Object Drawing.SolidBrush([Drawing.Color]::FromArgb(255, 32, 34, 42))
    $g.FillRectangle($dark, 0, $band, $width, $band)
    $dark.Dispose()
    foreach ($row in 0, 1) {
        $centre = $band * $row + $band / 2
        $x = $padding
        foreach ($size in $sizes) {
            $bitmap = New-IconBitmap -Size $size
            $g.DrawImage($bitmap, [int]$x, [int]($centre - $size / 2), $size, $size)
            $bitmap.Dispose()
            $x += $size + $padding
        }
    }
    $g.Dispose()
    $preview.Save($PreviewPath, [Drawing.Imaging.ImageFormat]::Png)
    $preview.Dispose()
    Write-Output "Wrote preview $PreviewPath"
    return
}

if (-not (Test-Path -LiteralPath $AssetDirectory)) {
    New-Item -ItemType Directory -Path $AssetDirectory -Force | Out-Null
}
Get-ChildItem -LiteralPath $AssetDirectory -Filter *.png | Remove-Item -Force

# Scaled variants of the manifest logos. Windows picks by display scale.
$scaled = @(
    @{ Name = "Square44x44Logo"; Base = 44; Wide = 0 },
    @{ Name = "Square71x71Logo"; Base = 71; Wide = 0 },
    @{ Name = "Square150x150Logo"; Base = 150; Wide = 0 },
    @{ Name = "Square310x310Logo"; Base = 310; Wide = 0 },
    @{ Name = "StoreLogo"; Base = 50; Wide = 0 },
    @{ Name = "Wide310x150Logo"; Base = 150; Wide = 310 }
)
foreach ($logo in $scaled) {
    foreach ($scale in 100, 125, 150, 200, 400) {
        $size = [int]($logo.Base * $scale / 100)
        $width = if ($logo.Wide) { [int]($logo.Wide * $scale / 100) } else { 0 }
        $path = Join-Path $AssetDirectory "$($logo.Name).scale-$scale.png"
        Save-Icon -Size $size -Width $width -Plated $true -UnplatedColor $Light -Path $path
    }
}

# App-list icons. Windows requires the plated form plus the two unplated
# variants, which it uses on light and dark shell surfaces.
foreach ($target in 16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256) {
    Save-Icon -Size $target -Width 0 -Plated $true -UnplatedColor $Light `
        -Path (Join-Path $AssetDirectory "Square44x44Logo.targetsize-$target.png")
    Save-Icon -Size $target -Width 0 -Plated $false -UnplatedColor ([Drawing.Color]::White) `
        -Path (Join-Path $AssetDirectory "Square44x44Logo.targetsize-${target}_altform-unplated.png")
    Save-Icon -Size $target -Width 0 -Plated $false -UnplatedColor ([Drawing.Color]::FromArgb(255, 32, 34, 42)) `
        -Path (Join-Path $AssetDirectory "Square44x44Logo.targetsize-${target}_altform-lightunplated.png")
}

$count = (Get-ChildItem -LiteralPath $AssetDirectory -Filter *.png).Count
Write-Output "Wrote $count package assets to $AssetDirectory"

# The .ico the executable embeds. Sizes are stored as PNG streams, which every
# supported Windows version reads.
$icoSizes = 16, 24, 32, 48, 64, 128, 256
$streams = @()
foreach ($size in $icoSizes) {
    $bitmap = New-IconBitmap -Size $size
    $stream = New-Object IO.MemoryStream
    $bitmap.Save($stream, [Drawing.Imaging.ImageFormat]::Png)
    $bitmap.Dispose()
    $streams += , $stream.ToArray()
    $stream.Dispose()
}

$file = [IO.File]::Create($IconPath)
$writer = New-Object IO.BinaryWriter $file
try {
    $writer.Write([UInt16]0)               # reserved
    $writer.Write([UInt16]1)               # type: icon
    $writer.Write([UInt16]$icoSizes.Count)
    $offset = 6 + 16 * $icoSizes.Count
    for ($i = 0; $i -lt $icoSizes.Count; $i++) {
        $size = $icoSizes[$i]
        # 256 is written as 0, the format's way of spelling it.
        $writer.Write([byte]($(if ($size -ge 256) { 0 } else { $size })))
        $writer.Write([byte]($(if ($size -ge 256) { 0 } else { $size })))
        $writer.Write([byte]0)             # palette entries
        $writer.Write([byte]0)             # reserved
        $writer.Write([UInt16]1)           # colour planes
        $writer.Write([UInt16]32)          # bits per pixel
        $writer.Write([UInt32]$streams[$i].Length)
        $writer.Write([UInt32]$offset)
        $offset += $streams[$i].Length
    }
    foreach ($bytes in $streams) {
        $writer.Write($bytes)
    }
}
finally {
    $writer.Dispose()
    $file.Dispose()
}
Write-Output "Wrote $IconPath ($($icoSizes.Count) sizes)"
