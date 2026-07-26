# 复现脚本：启动 Aether 并打开空文件夹，监控主进程与 LSP 子进程存活状态
$ErrorActionPreference = 'Continue'
$exe = "d:\Application\牧羊人编辑器\target\x86_64-pc-windows-msvc\debug\aether-app.exe"
$empty = "$env:TEMP\aether_empty_repro"
$launchArgs = @{ paths = @($empty); new_window = $false; goto = $null; wait = $false } | ConvertTo-Json -Compress

Write-Host "== 启动: $exe"
Write-Host "== 打开空文件夹: $empty"
Write-Host "== launch-args: $launchArgs"

$proc = Start-Process -FilePath $exe -ArgumentList @('--aether-launch-args', $launchArgs) -PassThru
Write-Host "== 主进程 PID: $($proc.Id) 启动时间: $(Get-Date -Format 'HH:mm:ss.fff')"

$deadline = (Get-Date).AddSeconds(90)
$lastChildren = @()
while ((Get-Date) -lt $deadline) {
    Start-Sleep -Seconds 3
    $now = Get-Date -Format 'HH:mm:ss'
    # 刷新主进程状态
    $alive = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
    if (-not $alive) {
        $proc.Refresh() 2>$null
        Write-Host "[$now] !!! 主进程已终止 ExitCode=$($proc.ExitCode)"
        break
    }
    # 枚举子进程
    $children = Get-CimInstance Win32_Process -Filter "ParentProcessId=$($proc.Id)" |
        Select-Object ProcessId, Name, CommandLine
    $childDesc = ($children | ForEach-Object { "$($_.Name)($($_.ProcessId))" }) -join ', '
    # 检测上一轮存在但本轮消失的子进程
    foreach ($old in $lastChildren) {
        if (-not ($children | Where-Object { $_.ProcessId -eq $old.ProcessId })) {
            Write-Host "[$now] !!! 子进程退出: $($old.Name)($($old.ProcessId)) cmd=$($old.CommandLine)"
        }
    }
    $lastChildren = $children
    Write-Host "[$now] 主进程存活 WS=$([math]::Round($alive.WorkingSet64/1MB,1))MB 子进程: [$childDesc]"
}

# 结束时清理（若主进程仍存活则关闭窗口）
$alive = Get-Process -Id $proc.Id -ErrorAction SilentlyContinue
if ($alive) {
    Write-Host "== 90 秒观察结束，主进程仍存活，关闭之"
    $alive.CloseMainWindow() | Out-Null
    Start-Sleep -Seconds 2
    if (-not $alive.HasExited) { $alive.Kill() }
} else {
    Write-Host "== 观察结束：主进程已不存在"
}
