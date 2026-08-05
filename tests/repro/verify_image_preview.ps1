# 图片预览验证：启动应用打开指定图片，激活窗口后截图
param(
    [string]$ImagePath = 'C:/Users/songd/AppData/Local/Temp/aether_img_preview/test.png',
    [string]$OutShot = 'C:\Users\songd\AppData\Local\Temp\aether_img_preview\shot.png',
    [int]$WaitSeconds = 4
)
Add-Type -AssemblyName System.Drawing
Add-Type @"
using System;
using System.Runtime.InteropServices;
public class U32B {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int cmd);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    public struct RECT { public int Left, Top, Right, Bottom; }
}
"@

Get-Process aether-app -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1

$imgDir = Split-Path $ImagePath -Parent
$trustFile = Join-Path $env:APPDATA 'Aether\trusted_folders.txt'
$trustDir = Split-Path $trustFile -Parent
if (-not (Test-Path $trustDir)) { New-Item -ItemType Directory -Force -Path $trustDir | Out-Null }
$existing = @()
if (Test-Path $trustFile) { $existing = Get-Content $trustFile -ErrorAction SilentlyContinue }
$keys = @( ($imgDir -replace '/', '\').ToLower(), ($imgDir -replace '\\', '/').ToLower() )
$need = $keys | Where-Object { $_ -notin $existing }
if ($need) { Add-Content $trustFile ($need -join "`n") }

$exe = 'd:\Application\牧羊人编辑器\target\x86_64-pc-windows-msvc\debug\aether-app.exe'
$json = '{"paths":["' + ($ImagePath -replace '\\', '/') + '"],"new_window":false,"goto":null,"wait":false}'
$argStr = '--aether-launch-args "' + ($json -replace '"', '\"') + '"'
Start-Process -FilePath $exe -ArgumentList $argStr | Out-Null

$proc = $null
for ($i = 0; $i -lt 40; $i++) {
    Start-Sleep -Milliseconds 500
    $proc = Get-Process aether-app -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
    if ($proc) { break }
}
if (-not $proc) { Write-Host "ERROR: 窗口未出现"; exit 1 }

Start-Sleep -Seconds $WaitSeconds
$hwnd = $proc.MainWindowHandle
# 恢复并置前
[U32B]::ShowWindow($hwnd, 9) | Out-Null   # SW_RESTORE
Start-Sleep -Milliseconds 300
[U32B]::SetForegroundWindow($hwnd) | Out-Null
Start-Sleep -Milliseconds 800

$fg = [U32B]::GetForegroundWindow()
$vis = [U32B]::IsWindowVisible($hwnd)
Write-Host "Aether hwnd=$hwnd visible=$vis 前台=$($fg -eq $hwnd)"

$r = New-Object U32B+RECT
[U32B]::GetWindowRect($hwnd, [ref]$r) | Out-Null
$w = $r.Right - $r.Left; $h = $r.Bottom - $r.Top
if ($w -le 0 -or $h -le 0) { Write-Host "ERROR: 窗口尺寸异常 $w x $h"; exit 1 }
$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.Left, $r.Top, 0, 0, (New-Object System.Drawing.Size $w, $h))
$bmp.Save($OutShot, [System.Drawing.Imaging.ImageFormat]::Png)
$g.Dispose(); $bmp.Dispose()
Write-Host "截图已保存: $OutShot  ($w x $h)"
