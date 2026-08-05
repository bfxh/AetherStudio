# 假终端输入暂存验证：Ctrl+J 后立即输入字符 → 截假终端（本地回显）→ 等真终端（暂存映射执行）
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class U32G {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
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
        [U32G]::keybd_event($vk, 0, 0, 0); Start-Sleep -Milliseconds 30
        [U32G]::keybd_event($vk, 0, [U32G]::KEYUP, 0); Start-Sleep -Milliseconds 30
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
[U32G]::ShowWindow($hwnd, 9) | Out-Null; Start-Sleep -Milliseconds 300
[U32G]::SetForegroundWindow($hwnd) | Out-Null; Start-Sleep -Milliseconds 500

# Ctrl+J 打开终端
[U32G]::keybd_event([U32G]::VK_CONTROL, 0, 0, 0); [U32G]::keybd_event([U32G]::VK_J, 0, 0, 0)
Start-Sleep -Milliseconds 150
[U32G]::keybd_event([U32G]::VK_J, 0, [U32G]::KEYUP, 0); [U32G]::keybd_event([U32G]::VK_CONTROL, 0, [U32G]::KEYUP, 0)

# 立即（假终端阶段）输入 "ls" + 回车
Start-Sleep -Milliseconds 300
Type-Str "ls"
[U32G]::keybd_event([U32G]::VK_RETURN, 0, 0, 0); Start-Sleep -Milliseconds 50
[U32G]::keybd_event([U32G]::VK_RETURN, 0, [U32G]::KEYUP, 0)

# 截假终端（应显示 PS...> ls 本地回显）
Start-Sleep -Milliseconds 400
$r = New-Object U32G+RECT
[U32G]::GetWindowRect($hwnd, [ref]$r) | Out-Null
Shot (Join-Path $dir 'term_fake_input.png') $r
Write-Host "假终端输入截图: term_fake_input.png"

# 等真终端就绪（暂存的 ls 应被映射执行，显示目录列表）
Start-Sleep -Seconds 4
[U32G]::GetWindowRect($hwnd, [ref]$r) | Out-Null
Shot (Join-Path $dir 'term_real_exec.png') $r
Write-Host "真终端执行截图: term_real_exec.png"
