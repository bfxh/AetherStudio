# 截取指定进程主窗口的屏幕区域（物理像素），用于 UI 视觉验证。
# 框架内请直接用 AetherTest.psm1 的 Save-AetherScreenshot；本脚本供临时手动截图。
param(
    [string]$ProcessName = "aether-app",
    [string]$OutPath = "d:\Application\牧羊人编辑器\tests\screenshots\window_capture.png"
)

Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win32Capture {
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")]
    public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")]
    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")]
    public static extern bool SetProcessDPIAware();
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
}
"@

[Win32Capture]::SetProcessDPIAware() | Out-Null

$proc = Get-Process $ProcessName -ErrorAction Stop | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
if (-not $proc) { Write-Error "未找到窗口"; exit 1 }

[Win32Capture]::ShowWindow($proc.MainWindowHandle, 9) | Out-Null  # SW_RESTORE
[Win32Capture]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
Start-Sleep -Milliseconds 1200

$rect = New-Object Win32Capture+RECT
[Win32Capture]::GetWindowRect($proc.MainWindowHandle, [ref]$rect) | Out-Null
$w = $rect.Right - $rect.Left
$h = $rect.Bottom - $rect.Top
if ($w -le 0 -or $h -le 0) { Write-Error "窗口尺寸无效"; exit 1 }

$bmp = New-Object System.Drawing.Bitmap($w, $h)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bmp.Size)
$g.Dispose()
$bmp.Save($OutPath, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Output "saved: $OutPath ($w x $h)"
