# Aether Studio 测试统一入口
#
# 用法：
#   pwsh -File tests\run_tests.ps1                 # 单元测试（默认）
#   pwsh -File tests\run_tests.ps1 -Suite unit     # cargo test 全工作区
#   pwsh -File tests\run_tests.ps1 -Suite gui      # 跑 tests\cases\*.tests.ps1（会操作鼠标键盘）
#   pwsh -File tests\run_tests.ps1 -Suite gui -Case explorer_inline_input
#   pwsh -File tests\run_tests.ps1 -Suite coverage # 插桩测试 + llvm-cov 报告
#   pwsh -File tests\run_tests.ps1 -Suite all      # unit + gui
#
# 目录结构：
#   framework\AetherTest.psm1   GUI 测试框架核心模块（窗口/输入/截图/断言/报告）
#   cases\*.tests.ps1           GUI 测试用例（可独立运行）
#   tools\coverage.ps1          覆盖率一体化脚本
#   repro\                      bug 复现脚本与报告
#   legacy\                     旧版 Python GUI 脚本（依赖 pip，仅存档）
#   reports\                    用例 JSON 报告输出
#   screenshots\<case>\         用例截图输出
param(
    [ValidateSet("unit", "gui", "coverage", "all")]
    [string]$Suite = "unit",
    [string]$Case,       # 只跑指定 GUI 用例（不含 .tests.ps1 后缀）
    [switch]$SkipBuild   # GUI 用例跳过 cargo build
)

$ErrorActionPreference = "Stop"
$projectRoot = (Resolve-Path "$PSScriptRoot\..").Path
$failures = 0

function Invoke-UnitSuite {
    Write-Host "`n########## 单元测试 (cargo test --workspace) ##########" -ForegroundColor Cyan
    Push-Location $projectRoot
    try {
        $env:CARGO_INCREMENTAL = "0"
        cargo test --workspace --no-fail-fast 2>&1 |
            Tee-Object -FilePath "$projectRoot\tests\reports\cargo_test.log" |
            Select-String -Pattern "^test result:|FAILED|panicked" | ForEach-Object { $_.Line } | Write-Host
        if ($LASTEXITCODE -ne 0) { return 1 }
        return 0
    } finally { Pop-Location }
}

function Invoke-GuiSuite {
    param([string]$Only)
    Write-Host "`n########## GUI 测试 (tests\cases) ##########" -ForegroundColor Cyan
    Write-Host "注意：GUI 用例会启动应用并模拟鼠标键盘，运行期间请勿操作。" -ForegroundColor Yellow
    $pattern = if ($Only) { "$Only.tests.ps1" } else { "*.tests.ps1" }
    $caseFiles = Get-ChildItem -Path "$PSScriptRoot\cases" -Filter $pattern -ErrorAction SilentlyContinue
    if (-not $caseFiles) {
        Write-Host "未找到用例: $pattern" -ForegroundColor Red
        return 1
    }
    $failed = 0
    foreach ($f in $caseFiles) {
        $argList = @("-NoProfile", "-File", $f.FullName)
        if ($SkipBuild) { $argList += "-SkipBuild" }
        & pwsh @argList
        if ($LASTEXITCODE -ne 0) { $failed++ }
    }
    return $failed
}

New-Item -ItemType Directory -Force -Path "$projectRoot\tests\reports" | Out-Null

switch ($Suite) {
    "unit" { $failures += Invoke-UnitSuite }
    "gui" { $failures += Invoke-GuiSuite -Only $Case }
    "coverage" { & pwsh -NoProfile -File "$PSScriptRoot\tools\coverage.ps1"; if ($LASTEXITCODE -ne 0) { $failures++ } }
    "all" {
        $failures += Invoke-UnitSuite
        $failures += Invoke-GuiSuite -Only $Case
    }
}

if ($failures -eq 0) {
    Write-Host "`n全部通过 ✔" -ForegroundColor Green
} else {
    Write-Host "`n存在失败项: $failures" -ForegroundColor Red
}
exit $failures
