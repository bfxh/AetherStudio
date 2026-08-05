# 验证：替换后真终端是否正常工作（手动敲 ls 看输出）
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class U32H {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint x, uint y, int d, uint e);
    [DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte s, uint f, uint e);
    public struct RECT { public int Left, Top, Right, Bottom; }
    public const uint KEYUP = 0x0002;
    public const byte VK_CONTROL = 0x11, VK_J = 0x4A, VK_RETURN = 0x0D;
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
function Type-Str([string]$s) {
    foreach ($ch in $s.ToCharArray()) {
        $vk = [byte][char]::ToUpperInvariant($ch)
        [U32H]::keybd_event($vk, 0, 0, 0); Start-Sleep -Milliseconds 40
        [U32H]::keybd_event($vk, 0, [U32H]::KEYUP, 0); Start-Sleep -Milliseconds 40
    }
}
$dir = 'C:\Users\songd\AppData\Local\Temp\aether_img_preview'
Get-Process aether-app -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1
$trustFile = Join-Path $env:APPDATA 'Aether\trusted_folders.txt'
$existing = @(); if (Test-Path $trustFile) { $existing = Get-Content $trustFile -ErrorAction SilentlyContinue }
$keys = @( ($dir -replace '/', '\').ToLower(), ($dir -replace '\\', '/').ToLower() )
$need = $keys | Where-Object { $_ -notin $existing }; if ($need) { Add-Content $trustFile ($need -join "`n") }
$exe = 'd:\Application\牧羊人编辑器\target\x86_64-pc-windows-msvc\debug\aether-app.exe'
$json = '{"paths":["' + ($dir -replace '\\', '/') + '"],"new_window":false,"goto":null,"wait":false}'
$argStr = '--aether-launch-args "' + ($json -replace '"', '\"') + '"'
Start-Process -FilePath $exe -ArgumentList $argStr | Out-Null
$proc = $null
for ($i = 0; $i -lt 40; $i++) { Start-Sleep -Milliseconds 500; $proc = Get-Process aether-app -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1; if ($proc) { break } }
if (-not $proc) { Write-Host "ERROR: 窗口未出现"; exit 1 }
Start-Sleep -Seconds 3
$hwnd = $proc.MainWindowHandle
[U32H]::ShowWindow($hwnd, 9) | Out-Null; Start-Sleep -Milliseconds 300
[U32H]::SetForegroundWindow($hwnd) | Out-Null; Start-Sleep -Milliseconds 500

# Ctrl+J 打开终端（不在假终端阶段输入，等真终端完全就绪）
[U32H]::keybd_event([U32H]::VK_CONTROL, 0, 0, 0); [U32H]::keybd_event([U32H]::VK_J, 0, 0, 0)
Start-Sleep -Milliseconds 150
[U32H]::keybd_event([U32H]::VK_J, 0, [U32H]::KEYUP, 0); [U32H]::keybd_event([U32H]::VK_CONTROL, 0, [U32H]::KEYUP, 0)

# 等真终端完全就绪（替换完成）
Start-Sleep -Seconds 4
# 点击终端面板中心确保聚焦（focused=true）
$r0 = New-Object U32H+RECT
[U32H]::GetWindowRect($hwnd, [ref]$r0) | Out-Null
$tx = $r0.Left + 900; $ty = $r0.Top + 720
[U32H]::SetCursorPos($tx, $ty) | Out-Null
Start-Sleep -Milliseconds 200
[U32H]::mouse_event(0x0002, 0, 0, 0, 0)  # LDOWN
Start-Sleep -Milliseconds 80
[U32H]::mouse_event(0x0004, 0, 0, 0, 0)  # LUP
Start-Sleep -Milliseconds 500
# 手动敲 ls + 回车
Type-Str "ls"
[U32H]::keybd_event([U32H]::VK_RETURN, 0, 0, 0); Start-Sleep -Milliseconds 60
[U32H]::keybd_event([U32H]::VK_RETURN, 0, [U32H]::KEYUP, 0)
# 等执行 + 输出
Start-Sleep -Seconds 2
$r = New-Object U32H+RECT
[U32H]::GetWindowRect($hwnd, [ref]$r) | Out-Null
Shot (Join-Path $dir 'term_manual_ls.png') $r
Write-Host "替换后手动 ls 截图: term_manual_ls.png"
