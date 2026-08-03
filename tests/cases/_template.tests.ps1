# =============================================================================
# AI 用例模板 — 复制本文件为 <功能名>.tests.ps1 后按注释填写
# =============================================================================
# 【AI 生成用例时的元数据约定】在文件头注释中填写以下字段，便于追溯与回归：
#   Target     : 被测功能/模块（如"资源管理器文件树"）
#   Feature    : 对应的需求/PRD 条目（如 REQ-P1-09）
#   Scenario   : 场景描述（一句话）
#   Expect     : 期望行为（可验证的断言要点）
#   AI-Prompt  : 生成该用例的原始指令（可选）
#   Layout     : 依赖的布局常量（坐标计算基准）
# =============================================================================

param([switch]$SkipBuild)

Import-Module "$PSScriptRoot\..\framework\AetherTest.psm1" -Force
Import-Module "$PSScriptRoot\..\framework\AetherAi.psm1" -Force   # AI 协作层（按需）

$L = Get-AetherLayoutConstants    # 布局常量：TITLE_BAR/ACTIVITY_W/SIDEBAR_W/HEADER_H/ROW_H

Start-TestCase "template_case"    # ← 用例名（与文件名一致，用于报告/截图/诊断包命名）

$ws = $null
$proc = $null
try {
    if (-not $SkipBuild) { Build-AetherApp }

    # ---- 测试准备：临时工作区（AI 用 @{ '相对路径' = '内容' } 声明所需文件） ----
    $ws = New-AetherTestWorkspace -Files @{
        "main.rs"      = 'fn main() { println!("hello"); }'
        "notes.md"     = "# notes"
    }

    # ---- 启动应用（自动信任工作区 + new_window 隔离，避免干扰用户窗口） ----
    $proc = Start-AetherApp -Folder $ws
    # 隔离到 (40,40) 避免与用户已开的窗口重叠（截图/真实鼠标需要）
    $win = Get-AetherWindow -Process $proc -Isolate
    # 坐标换算：窗口内物理坐标 = 名义值 × Scale2
    $k = $win.Scale2
    function NX([double]$v) { [int]($v * $k) }
    # 文件树第 i 个节点行中心（目录优先 + 字母序；行高 ROW_H）
    function RowCenterY([int]$i) { NX ($L.TITLE_BAR + $L.HEADER_H + 6 + $L.ROW_H * ($i + 1) + $L.ROW_H / 2) }
    $rowLabelX = NX ($L.ACTIVITY_W + 10 + 12 + 30)

    # ---- 测试步骤（键盘输入用 PostMessage 注入，不依赖前台焦点） ----
    Invoke-TestStep "点击第一个文件打开" {
        Send-AetherClickMsg -Hwnd $win.Hwnd -X $rowLabelX -Y (RowCenterY 0)
        # 用 hit regions 状态栏语言断言打开成功（status_message 不写日志）
        $deadline = (Get-Date).AddSeconds(8)
        while ((Get-Date) -lt $deadline) {
            Start-Sleep -Milliseconds 300
            if (@(Read-AetherHitRegions -ActionLike "status:Rust").Count -gt 0) { break }
        }
        Assert-Condition (@(Read-AetherHitRegions -ActionLike "status:Rust").Count -gt 0) "状态栏切换为 Rust（文件已打开）"
        Save-AetherScreenshot -Window $win -Name "opened" | Out-Null
    }

    Invoke-TestStep "切换标签页零开销" {
        Send-AetherClickMsg -Hwnd $win.Hwnd -X $rowLabelX -Y (RowCenterY 1)
        Start-Sleep -Milliseconds 500
        Save-AetherScreenshot -Window $win -Name "switched" | Out-Null
    }

    # ---- 失败处理：断言异常会被 Invoke-TestStep 捕获，无需手动 try/catch ----
    # ---- 失败后（可选）：New-AetherDiagBundle -CaseName "template_case" 打包诊断材料 ----

} finally {
    if ($proc) { Stop-AetherApp -Process $proc }
    if ($ws)   { Remove-AetherTestWorkspace -Path $ws }
}

exit (Complete-TestCase)
