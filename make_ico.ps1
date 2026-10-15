Add-Type -AssemblyName System.Drawing
$bmp = New-Object System.Drawing.Bitmap 32, 32
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.Clear([System.Drawing.Color]::FromArgb(26, 35, 126))
$penCyan = New-Object System.Drawing.Pen([System.Drawing.Color]::Cyan, 2)
$g.DrawRectangle($penCyan, 1, 1, 29, 29)
$brushWhite = [System.Drawing.Brushes]::White
$g.FillRectangle($brushWhite, 5, 5, 7, 7)
$g.FillRectangle($brushWhite, 19, 5, 7, 7)
$g.FillRectangle($brushWhite, 5, 19, 7, 7)
$g.FillRectangle($brushWhite, 18, 18, 5, 5)
$g.FillRectangle($brushWhite, 24, 23, 3, 3)
$penRed = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(255, 61, 0), 2)
$g.DrawLine($penRed, 3, 16, 28, 16)
$hIcon = $bmp.GetHicon()
$icon = [System.Drawing.Icon]::FromHandle($hIcon)
$fs = New-Object System.IO.FileStream('app.ico', [System.IO.FileMode]::Create)
$icon.Save($fs)
$fs.Close()
