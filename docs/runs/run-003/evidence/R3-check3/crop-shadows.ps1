$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
# Early captures used the outer window rectangle. Retain only its client pixels:
# 11 px side/bottom shadow and 45 px titlebar/top inset, matching the later client captures.
foreach ($name in 'dashboard','close-option','pending-attention','pending-decision','interrupted-list','interrupted-addon') {
    $path = "$PSScriptRoot\$name.png"
    $original = [Drawing.Bitmap]::new($path)
    if ($original.Width -ne 1582 -or $original.Height -ne 1136) {
        $original.Dispose()
        throw "Unexpected source size for $name; no crop performed"
    }
    $crop = $original.Clone([Drawing.Rectangle]::new(11,45,1560,1080),$original.PixelFormat)
    $original.Dispose()
    try { $crop.Save($path,[Drawing.Imaging.ImageFormat]::Png) }
    finally { $crop.Dispose() }
    Write-Host "Cropped $name to original client-area pixels only"
}
