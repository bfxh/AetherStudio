# AetherTest.psm1 — Aether Studio GUI/进程测试框架核心模块
#
# 能力：应用生命周期（构建/启动/停止）、窗口管理、鼠标键盘模拟、
#       截图、临时测试工作区、断言与用例报告（JSON + 控制台）。
#
# 用例协议（tests/cases/*.tests.ps1）：
#   Import-Module "$PSScriptRoot\..\framework\AetherTest.psm1" -Force
#   Start-TestCase "case_name"
#   Invoke-TestStep "步骤描述" { ...; Assert-Condition ... }
#   exit (Complete-TestCase)
#
# 坐标说明：本应用渲染物理尺寸 = 名义值(布局常量) × dpi_scale²（150% 屏为
# 2.25 倍），Get-AetherScale 返回该平方系数，用例按名义常量计算点击点。

Set-StrictMode -Version Latest

$script:ProjectRoot = (Resolve-Path "$PSScriptRoot\..\..").Path
$script:AppExe = Join-Path $script:ProjectRoot "target\x86_64-pc-windows-msvc\debug\aether-app.exe"
$script:ScreenshotDir = Join-Path $script:ProjectRoot "tests\screenshots"
$script:ReportDir = Join-Path $script:ProjectRoot "tests\reports"

Add-Type -AssemblyName System.Drawing

if (-not ("AetherWin32" -as [type])) {
    Add-Type @"
using System;
using System.Runtime.InteropServices;
public class AetherWin32 {
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr hWnd, out RECT lpRect);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr hWnd);
    [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern void mouse_event(uint dwFlags, uint dx, uint dy, uint dwData, UIntPtr dwExtraInfo);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr hWnd);
    public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }
    public const uint LEFTDOWN = 0x0002, LEFTUP = 0x0004, RIGHTDOWN = 0x0008, RIGHTUP = 0x0010;
}
"@
}
[AetherWin32]::SetProcessDPIAware() | Out-Null

# ---------------------------------------------------------------- 应用生命周期

function Build-AetherApp {
    <# 构建 debug 版 aether-app（增量，已最新时秒回） #>
    Push-Location $script:ProjectRoot
    try {
        cargo build -p aether-win32 2>&1 | Select-Object -Last 1 | Write-Host
        if ($LASTEXITCODE -ne 0) { throw "cargo build 失败" }
    } finally { Pop-Location }
}

function Start-AetherApp {
    <# 启动应用并等待主窗口就绪。-Folder 指定启动时打开的工作区文件夹 #>
    param(
        [string]$Folder,
        [int]$TimeoutSec = 15
    )
    if (-not (Test-Path $script:AppExe)) { throw "未找到 $script:AppExe，请先 Build-AetherApp" }
    if ($Folder) {
        # 与 aether-cli 相同的启动参数协议（见 tests/repro/repro_empty_folder.ps1）
        $launchArgs = @{ paths = @($Folder); new_window = $false; goto = $null; wait = $false } | ConvertTo-Json -Compress
        $proc = Start-Process -FilePath $script:AppExe -ArgumentList @('--aether-launch-args', $launchArgs) -PassThru
    } else {
        $proc = Start-Process -FilePath $script:AppExe -PassThru
    }
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        Start-Sleep -Milliseconds 500
        $proc.Refresh()
        if ($proc.HasExited) { throw "应用启动后立即退出 ExitCode=$($proc.ExitCode)" }
        if ($proc.MainWindowHandle -ne 0) {
            Start-Sleep -Milliseconds 1500   # 等首帧渲染稳定
            return $proc
        }
    }
    throw "等待主窗口超时（${TimeoutSec}s）"
}

function Stop-AetherApp {
    param([Parameter(Mandatory)]$Process)
    if ($Process -and -not $Process.HasExited) {
        $Process.CloseMainWindow() | Out-Null
        Start-Sleep -Seconds 1
        if (-not $Process.HasExited) { $Process.Kill() }
    }
}

# ---------------------------------------------------------------- 窗口与几何

function Get-AetherWindow {
    <# 恢复并前置主窗口，返回 @{ Hwnd; Rect(物理像素); Dpi; Scale; Scale2 } #>
    param($Process)
    if (-not $Process) {
        $Process = Get-Process aether-app -ErrorAction Stop |
            Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
    }
    $hwnd = $Process.MainWindowHandle
    [AetherWin32]::ShowWindow($hwnd, 9) | Out-Null   # SW_RESTORE
    [AetherWin32]::SetForegroundWindow($hwnd) | Out-Null
    Start-Sleep -Milliseconds 800
    $rect = New-Object AetherWin32+RECT
    [AetherWin32]::GetWindowRect($hwnd, [ref]$rect) | Out-Null
    $dpi = [AetherWin32]::GetDpiForWindow($hwnd)
    if ($dpi -le 0) { $dpi = 96 }
    $s = $dpi / 96.0
    [pscustomobject]@{
        Hwnd   = $hwnd
        Rect   = $rect
        Dpi    = $dpi
        Scale  = $s
        # 本应用布局常量的物理换算系数（渲染目标 DPI 与手动缩放叠加）
        Scale2 = $s * $s
    }
}

# ---------------------------------------------------------------- 输入模拟

function Invoke-AetherClick {
    <# 在窗口内相对坐标（物理像素）处单击。-Right 为右键 #>
    param(
        [Parameter(Mandatory)]$Window,
        [Parameter(Mandatory)][int]$X,
        [Parameter(Mandatory)][int]$Y,
        [switch]$Right
    )
    $px = $Window.Rect.Left + $X
    $py = $Window.Rect.Top + $Y
    [AetherWin32]::SetCursorPos($px, $py) | Out-Null
    Start-Sleep -Milliseconds 120
    if ($Right) {
        [AetherWin32]::mouse_event([AetherWin32]::RIGHTDOWN, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 60
        [AetherWin32]::mouse_event([AetherWin32]::RIGHTUP, 0, 0, 0, [UIntPtr]::Zero)
    } else {
        [AetherWin32]::mouse_event([AetherWin32]::LEFTDOWN, 0, 0, 0, [UIntPtr]::Zero)
        Start-Sleep -Milliseconds 60
        [AetherWin32]::mouse_event([AetherWin32]::LEFTUP, 0, 0, 0, [UIntPtr]::Zero)
    }
    Start-Sleep -Milliseconds 400
}

function Send-AetherKeys {
    <# 发送 SendKeys 序列（{F2}/{ESC}/{ENTER} 等控制键原样传入） #>
    param([Parameter(Mandatory)][string]$Keys, [int]$DelayMs = 400)
    $shell = New-Object -ComObject WScript.Shell
    $shell.SendKeys($Keys)
    Start-Sleep -Milliseconds $DelayMs
}

function Send-AetherText {
    <# 发送字面文本：自动转义 SendKeys 元字符 +^%~(){}[] #>
    param([Parameter(Mandatory)][string]$Text, [int]$DelayMs = 400)
    $escaped = ($Text.ToCharArray() | ForEach-Object {
        if ($_ -match '[+^%~(){}\[\]]') { "{$_}" } else { "$_" }
    }) -join ''
    Send-AetherKeys -Keys $escaped -DelayMs $DelayMs
}

# ---------------------------------------------------------------- 截图

function Save-AetherScreenshot {
    <# 截取窗口区域，保存到 tests/screenshots/<用例名>/<Name>.png，返回路径 #>
    param(
        [Parameter(Mandatory)]$Window,
        [Parameter(Mandatory)][string]$Name
    )
    $sub = if ($script:Case) { Join-Path $script:ScreenshotDir $script:Case.Name } else { $script:ScreenshotDir }
    New-Item -ItemType Directory -Force -Path $sub | Out-Null
    $r = $Window.Rect
    $w = $r.Right - $r.Left
    $h = $r.Bottom - $r.Top
    if ($w -le 0 -or $h -le 0) { throw "窗口尺寸无效 ${w}x${h}" }
    $bmp = New-Object System.Drawing.Bitmap($w, $h)
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.CopyFromScreen($r.Left, $r.Top, 0, 0, $bmp.Size)
    $g.Dispose()
    $path = Join-Path $sub "$Name.png"
    $bmp.Save($path, [System.Drawing.Imaging.ImageFormat]::Png)
    $bmp.Dispose()
    if ($script:Case) { $script:Case.Screenshots += $path }
    Write-Host "  [截图] $path"
    return $path
}

# ---------------------------------------------------------------- 临时工作区

function New-AetherTestWorkspace {
    <# 创建临时工作区目录；-Files 哈希表 @{ '相对路径' = '内容' } #>
    param([hashtable]$Files = @{})
    $dir = Join-Path $env:TEMP ("aether_test_ws_" + [guid]::NewGuid().ToString("N").Substring(0, 8))
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    foreach ($rel in $Files.Keys) {
        $full = Join-Path $dir $rel
        New-Item -ItemType Directory -Force -Path (Split-Path $full -Parent) | Out-Null
        Set-Content -Path $full -Value $Files[$rel] -Encoding UTF8 -NoNewline
    }
    return $dir
}

function Remove-AetherTestWorkspace {
    param([Parameter(Mandatory)][string]$Path)
    # 只清理本框架创建的临时目录，防止误删
    if ($Path -like (Join-Path $env:TEMP "aether_test_ws_*")) {
        Remove-Item -Recurse -Force $Path -ErrorAction SilentlyContinue
    }
}

# ---------------------------------------------------------------- 用例与断言

$script:Case = $null

function Start-TestCase {
    param([Parameter(Mandatory)][string]$Name)
    $script:Case = [pscustomobject]@{
        Name        = $Name
        StartedAt   = (Get-Date).ToString("o")
        Steps       = @()
        Screenshots = @()
        Failed      = 0
    }
    Write-Host "`n=== 用例: $Name ===" -ForegroundColor Cyan
}

function Invoke-TestStep {
    <# 执行一个测试步骤；异常记为失败但不中断后续步骤 #>
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][scriptblock]$Body
    )
    Write-Host "-- $Name" -ForegroundColor Yellow
    $step = @{ name = $Name; ok = $true; error = $null }
    try {
        & $Body
    } catch {
        $step.ok = $false
        $step.error = "$_"
        $script:Case.Failed++
        Write-Host "   失败: $_" -ForegroundColor Red
    }
    $script:Case.Steps += $step
}

function Assert-Condition {
    param(
        [Parameter(Mandatory)][bool]$Condition,
        [Parameter(Mandatory)][string]$Message
    )
    if (-not $Condition) { throw "断言失败: $Message" }
    Write-Host "   ✓ $Message" -ForegroundColor Green
}

function Assert-PathExists {
    param([Parameter(Mandatory)][string]$Path)
    Assert-Condition (Test-Path $Path) "路径存在: $Path"
}

function Assert-PathMissing {
    param([Parameter(Mandatory)][string]$Path)
    Assert-Condition (-not (Test-Path $Path)) "路径不存在: $Path"
}

function Complete-TestCase {
    <# 输出 JSON 报告与汇总，返回失败步骤数（用作退出码） #>
    $c = $script:Case
    if (-not $c) { return 0 }
    New-Item -ItemType Directory -Force -Path $script:ReportDir | Out-Null
    $report = Join-Path $script:ReportDir "$($c.Name).json"
    $c | Add-Member -NotePropertyName FinishedAt -NotePropertyValue (Get-Date).ToString("o") -Force
    $c | ConvertTo-Json -Depth 5 | Set-Content -Path $report -Encoding UTF8
    $total = $c.Steps.Count
    $failed = $c.Failed
    $color = if ($failed -eq 0) { "Green" } else { "Red" }
    Write-Host "=== $($c.Name): $($total - $failed)/$total 步通过，报告 $report ===" -ForegroundColor $color
    $script:Case = $null
    return $failed
}

Export-ModuleMember -Function Build-AetherApp, Start-AetherApp, Stop-AetherApp,
    Get-AetherWindow, Invoke-AetherClick, Send-AetherKeys, Send-AetherText,
    Save-AetherScreenshot, New-AetherTestWorkspace, Remove-AetherTestWorkspace,
    Start-TestCase, Invoke-TestStep, Assert-Condition, Assert-PathExists,
    Assert-PathMissing, Complete-TestCase
