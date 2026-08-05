# 打开文件后模拟鼠标滚轮小幅滚动（触发 scroll_y 非整数倍行高），截图验证首行不越界
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class U32D {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint dx, uint dy, int data, uint extra);
    public struct RECT { public int Left, Top, Right, Bottom; }
    public const uint WHEEL = 0x0800;
}
"@

$dir = 'C:\Users\songd\AppData\Local\Temp\aether_img_preview'
$file = Join-Path $dir 'scroll_test.txt'
# 文件已存在则跳过重建
if (-not (Test-Path $file)) {
    $lines = 1..50 | ForEach-Object { "line $_ : this.canvas = document.getElementById('gameCanvas');" }
    Set-Content -Path $file -Value ($lines -join "`n") -Encoding UTF8
}

Get-Process aether-app -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1
$trustFile = Join-Path $env:APPDATA 'Aether\trusted_folders.txt'
$existing = @()
if (Test-Path $trustFile) { $existing = Get-Content $trustFile -ErrorAction SilentlyContinue }
$keys = @( ($dir -replace '/', '\').ToLower(), ($dir -replace '\\', '/').ToLower() )
$need = $keys | Where-Object { $_ -notin $existing }
if ($need) { Add-Content $trustFile ($need -join "`n") }

$exe = 'd:\Application\牧羊人编辑器\target\x86_64-pc-windows-msvc\debug\aether-app.exe'
$json = '{"paths":["' + ($file -replace '\\', '/') + '"],"new_window":false,"goto":null,"wait":false}'
$argStr = '--aether-launch-args "' + ($json -replace '"', '\"') + '"'
Start-Process -FilePath $exe -ArgumentList $argStr | Out-Null

$proc = $null
for ($i = 0; $i -lt 40; $i++) {
    Start-Sleep -Milliseconds 500
    $proc = Get-Process aether-app -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
    if ($proc) { break }
}
if (-not $proc) { Write-Host "ERROR: 窗口未出现"; exit 1 }
Start-Sleep -Seconds 4
$hwnd = $proc.MainWindowHandle
[U32D]::ShowWindow($hwnd, 9) | Out-Null
Start-Sleep -Milliseconds 300
[U32D]::SetForegroundWindow($hwnd) | Out-Null
Start-Sleep -Milliseconds 800

$r = New-Object U32D+RECT
[U32D]::GetWindowRect($hwnd, [ref]$r) | Out-Null
# 编辑区中心（屏幕坐标）：窗口左 + 编辑区偏移。编辑区约从窗口 x+640, y+170 起
$cx = $r.Left + 900
$cy = $r.Top + 400
[U32D]::SetCursorPos($cx, $cy) | Out-Null
Start-Sleep -Milliseconds 300
# 向下滚动 2 格（每格 120，编辑器通常按行滚动，可能产生非整数倍偏移）
[U32D]::mouse_event([U32D]::WHEEL, 0, 0, -120, 0)
Start-Sleep -Milliseconds 400
[U32D]::mouse_event([U32D]::WHEEL, 0, 0, -120, 0)
Start-Sleep -Milliseconds 800

$w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size $w, $h))
$out = Join-Path $dir 'shot_scrolled.png'
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host "截图已保存: $out"
