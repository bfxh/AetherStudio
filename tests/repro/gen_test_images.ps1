# 生成多格式测试图片（JPG/BMP/GIF），PNG 用项目 assets
Add-Type -AssemblyName System.Drawing
$dir = 'C:\Users\songd\AppData\Local\Temp\aether_img_preview'
New-Item -ItemType Directory -Force -Path $dir | Out-Null

function New-TestBitmap([int]$w, [int]$h, [string]$label) {
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    # 渐变背景
    for ($i = 0; $i -lt $h; $i++) {
        $c = [System.Drawing.Color]::FromArgb(255, [int](120 + 100 * $i / $h), [int](60 + 80 * $i / $h), 200)
        $pen = New-Object System.Drawing.Pen $c
        $g.DrawLine($pen, 0, $i, $w, $i)
        $pen.Dispose()
    }
    # 圆形
    $brush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 255, 180, 0))
    $g.FillEllipse($brush, [int]($w*0.2), [int]($h*0.2), [int]($w*0.4), [int]($h*0.4))
    $brush.Dispose()
    # 文字
    $font = New-Object System.Drawing.Font('Arial', [int]($h/8))
    $tb = [System.Drawing.Brushes]::White
    $g.DrawString($label, $font, $tb, 20, [int]($h*0.7))
    $font.Dispose()
    $g.Dispose()
    return $bmp
}

# JPG
$b1 = New-TestBitmap 640 480 'JPEG Test'
$b1.Save((Join-Path $dir 'test.jpg'), [System.Drawing.Imaging.ImageFormat]::Jpeg)
$b1.Dispose()
# BMP
$b2 = New-TestBitmap 500 400 'BMP Test'
$b2.Save((Join-Path $dir 'test.bmp'), [System.Drawing.Imaging.ImageFormat]::Bmp)
$b2.Dispose()
# GIF
$b3 = New-TestBitmap 400 300 'GIF Test'
$b3.Save((Join-Path $dir 'test.gif'), [System.Drawing.Imaging.ImageFormat]::Gif)
$b3.Dispose()
# PNG（也生成一张，尺寸不同于 assets）
$b4 = New-TestBitmap 800 600 'PNG Test'
$b4.Save((Join-Path $dir 'test.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$b4.Dispose()

Get-ChildItem $dir | Select-Object Name, Length | Format-Table -AutoSize
Write-Host "DIR=$dir"
