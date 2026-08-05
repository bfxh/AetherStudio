# 复现：文件编辑区点击两次才能移动光标
# 策略：通过 --aether-launch-args JSON 直接让实例打开测试文件（路径用正斜杠，
# 避免 Windows 反斜杠在 JSON/命令行双层转义中的陷阱）
$ErrorActionPreference = 'Stop'

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class Win {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr i);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int n);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr a, int x, int y, int cx, int cy, uint f);
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int L, T, R, B; }
    public const uint DOWN = 0x0002, UP = 0x0004;
    public const uint SWP_NOZORDER = 0x0004, SWP_NOACTIVATE = 0x0010;
    public static void Click(int x, int y) {
        SetCursorPos(x, y);
        System.Threading.Thread.Sleep(200);
        mouse_event(DOWN, 0, 0, 0, IntPtr.Zero);
        System.Threading.Thread.Sleep(100);
        mouse_event(UP, 0, 0, 0, IntPtr.Zero);
    }
}
"@
Add-Type -AssemblyName System.Drawing

function Capture($hwnd, $path) {
    $r = New-Object Win+RECT
    [Win]::GetWindowRect($hwnd, [ref]$r) | Out-Null
    $w = $r.R - $r.L; $h = $r.B - $r.T
    $bmp = New-Object System.Drawing.Bitmap $w, $h
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $h))
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $g.Dispose(); $bmp.Dispose()
}

$ws = "$env:TEMP\aether_click_repro"
New-Item -ItemType Directory -Force -Path $ws | Out-Null
1..30 | ForEach-Object { "line $_ : hello world aether editor test" } | Set-Content "$ws\test.txt" -Encoding UTF8

$exe = "d:\Application\牧羊人编辑器\target\x86_64-pc-windows-msvc\debug\aether-app.exe"
# 清理旧实例（单实例应用会把参数转发给旧进程，导致跑的不是新构建）
Get-Process aether-app -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Seconds 1

# 预信任临时工作区，避免 open_folder 弹"工作区信任"模态框阻塞复现
# （模态框在持有 borrow_mut 期间泵消息，会引发 RefCell 重入 panic 风暴）
$trustFile = "$env:APPDATA\Aether\trusted_folders.txt"
$wsLowerFwd = ($ws -replace '\\', '/').ToLower()
$wsLowerBack = $ws.ToLower()
$existing = if (Test-Path $trustFile) { Get-Content $trustFile } else { @() }
$need = @($wsLowerFwd, $wsLowerBack) | Where-Object { $_ -notin $existing }
if ($need) { Add-Content $trustFile ($need -join "`n") }
Write-Host "信任列表已包含: $wsLowerFwd"

# 构造启动参数 JSON：路径用正斜杠（Windows API 接受，且避免 JSON 反斜杠转义）
$fileFwd = ("$ws\test.txt") -replace '\\', '/'
$json = '{"paths":["' + $fileFwd + '"],"new_window":false,"goto":null,"wait":false}'
# Start-Process 不会自动给含引号的参数加外层引号，需手动转义：
# 内层 " 变成 \"，整体再包一层 "，否则 CommandLineToArgvW 会把 JSON 引号当分隔符
$argStr = '--aether-launch-args "' + ($json -replace '"', '\"') + '"'
Write-Host "启动参数: $argStr"

# 记录启动前日志大小，便于后面只看新产生的日志
$logFile = Get-ChildItem "$env:TEMP\Aether\logs\aether.*" -ErrorAction SilentlyContinue |
    Sort-Object LastWriteTime -Descending | Select-Object -First 1
$logOffset = 0
if ($logFile) { $logOffset = $logFile.Length }

# 直接带启动参数启动（第一个实例，无转发问题）
Start-Process -FilePath $exe -ArgumentList $argStr | Out-Null

# 轮询等待窗口句柄出现（debug 构建启动较慢）
$hwnd = [IntPtr]::Zero
for ($i = 0; $i -lt 60; $i++) {
    Start-Sleep -Milliseconds 500
    $p = Get-Process aether-app -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($p -and $p.MainWindowHandle -ne [IntPtr]::Zero) { $hwnd = $p.MainWindowHandle; break }
}
if ($hwnd -eq [IntPtr]::Zero) { throw "未找到 aether-app 主窗口" }
[Win]::ShowWindow($hwnd, 9) | Out-Null   # SW_RESTORE
Start-Sleep -Seconds 2
[Win]::SetForegroundWindow($hwnd) | Out-Null
Start-Sleep -Seconds 3

$r = New-Object Win+RECT
[Win]::GetWindowRect($hwnd, [ref]$r) | Out-Null
Write-Host "窗口区域: L=$($r.L) T=$($r.T) R=$($r.R) B=$($r.B)"

$shotDir = "$ws\shots"
New-Item -ItemType Directory -Force -Path $shotDir | Out-Null
Remove-Item "$shotDir\*.png" -Force -ErrorAction SilentlyContinue

# 基线截图（文件应已打开）
[Win]::SetForegroundWindow($hwnd) | Out-Null
Start-Sleep -Milliseconds 500
Capture $hwnd "$shotDir\00_baseline.png"

# 第一次点击：编辑区中部偏上（应落在文件前几行）
$x1 = $r.L + 500; $y1 = $r.T + 180
Write-Host "第一次点击: ($x1, $y1)"
[Win]::Click($x1, $y1)
Start-Sleep -Milliseconds 900
Capture $hwnd "$shotDir\01_after_click1.png"

# 第二次点击：换位置（向右、向下）
$x2 = $r.L + 750; $y2 = $r.T + 260
Write-Host "第二次点击: ($x2, $y2)"
[Win]::Click($x2, $y2)
Start-Sleep -Milliseconds 900
Capture $hwnd "$shotDir\02_after_click2.png"

# 第三次点击：再换位置
$x3 = $r.L + 420; $y3 = $r.T + 320
Write-Host "第三次点击: ($x3, $y3)"
[Win]::Click($x3, $y3)
Start-Sleep -Milliseconds 900
Capture $hwnd "$shotDir\03_after_click3.png"

Write-Host "`n===== 本次运行新增日志中的 LBD 诊断 ====="
$logFile = Get-ChildItem "$env:TEMP\Aether\logs\aether.*" | Sort-Object LastWriteTime -Descending | Select-Object -First 1
$fs = [System.IO.File]::Open($logFile.FullName, 'Open', 'Read', 'ReadWrite')
$fs.Seek($logOffset, 'Begin') | Out-Null
$reader = New-Object System.IO.StreamReader($fs, [System.Text.Encoding]::UTF8)
$newLog = $reader.ReadToEnd()
$reader.Dispose(); $fs.Dispose()
$newLog -split "`n" | Where-Object { $_ -match 'LBD|load_file|打开' } | Select-Object -Last 30

Write-Host "`n截图目录: $shotDir"
