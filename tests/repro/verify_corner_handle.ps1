# Corner Handle 实测：Ctrl+J 开底部面板 → 拖拽左下拐角 → 对比前后截图
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class U32E {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, int d, uint e);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte s, uint f, uint e);
    [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int vk);
    public struct RECT { public int Left, Top, Right, Bottom; }
    public const uint LDOWN = 0x0002, LUP = 0x0004;
    public const uint KEYUP = 0x0002;
    public const byte VK_CONTROL = 0x11, VK_J = 0x4A;
}
"@
function Shot([string]$path, $r) {
    $w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size $w, $h))
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
}

$dir = 'C:\Users\songd\AppData\Local\Temp\aether_img_preview'
$file = Join-Path $dir 'scroll_test.txt'
Get-Process aether-app -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1
$trustFile = Join-Path $env:APPDATA 'Aether\trusted_folders.txt'
$existing = @(); if (Test-Path $trustFile) { $existing = Get-Content $trustFile -ErrorAction SilentlyContinue }
$keys = @( ($dir -replace '/', '\').ToLower(), ($dir -replace '\\', '/').ToLower() )
$need = $keys | Where-Object { $_ -notin $existing }; if ($need) { Add-Content $trustFile ($need -join "`n") }

$exe = 'd:\Application\牧羊人编辑器\target\x86_64-pc-windows-msvc\debug\aether-app.exe'
$json = '{"paths":["' + ($file -replace '\\', '/') + '"],"new_window":false,"goto":null,"wait":false}'
$argStr = '--aether-launch-args "' + ($json -replace '"', '\"') + '"'
Start-Process -FilePath $exe -ArgumentList $argStr | Out-Null
$proc = $null
for ($i = 0; $i -lt 40; $i++) { Start-Sleep -Milliseconds 500; $proc = Get-Process aether-app -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1; if ($proc) { break } }
if (-not $proc) { Write-Host "ERROR: 窗口未出现"; exit 1 }
Start-Sleep -Seconds 4
$hwnd = $proc.MainWindowHandle
[U32E]::ShowWindow($hwnd, 9) | Out-Null; Start-Sleep -Milliseconds 300
[U32E]::SetForegroundWindow($hwnd) | Out-Null; Start-Sleep -Milliseconds 800

# Ctrl+J 打开底部面板
[U32E]::keybd_event([U32E]::VK_CONTROL, 0, 0, 0)
[U32E]::keybd_event([U32E]::VK_J, 0, 0, 0)
Start-Sleep -Milliseconds 200
[U32E]::keybd_event([U32E]::VK_J, 0, [U32E]::KEYUP, 0)
[U32E]::keybd_event([U32E]::VK_CONTROL, 0, [U32E]::KEYUP, 0)
Start-Sleep -Seconds 2

$r = New-Object U32E+RECT
[U32E]::GetWindowRect($hwnd, [ref]$r) | Out-Null
Shot (Join-Path $dir 'corner_before.png') $r
Write-Host "拖拽前截图: corner_before.png  窗口: L=$($r.Left) T=$($r.Top) R=$($r.Right) B=$($r.Bottom)"

# 左下拐角屏幕坐标：从 before 截图实测（1280x800 客户区）
# 侧边栏右缘 ≈ 截图 x=625，底部面板顶缘 ≈ 截图 y=655
# 屏幕坐标 = 窗口左上 + 客户区坐标（自绘窗口客户区≈窗口矩形）
$cornerX = $r.Left + 625
$cornerY = $r.Top + 655
Write-Host "拐角屏幕坐标: ($cornerX, $cornerY)"

# 拖拽：按下 → 右下移动 60px → 释放
[U32E]::SetCursorPos($cornerX, $cornerY) | Out-Null
Start-Sleep -Milliseconds 400
[U32E]::mouse_event([U32E]::LDOWN, 0, 0, 0, 0)
Start-Sleep -Milliseconds 200
for ($i = 1; $i -le 6; $i++) {
    [U32E]::SetCursorPos($cornerX + $i * 10, $cornerY + $i * 10) | Out-Null
    Start-Sleep -Milliseconds 60
}
Start-Sleep -Milliseconds 200
[U32E]::mouse_event([U32E]::LUP, 0, 0, 0, 0)
Start-Sleep -Seconds 1

[U32E]::GetWindowRect($hwnd, [ref]$r) | Out-Null
Shot (Join-Path $dir 'corner_after.png') $r
Write-Host "拖拽后截图: corner_after.png"
