# 框架脚本语法自检（供 CI/本地快速验证，不执行任何 GUI 操作）
Set-Location (Resolve-Path "$PSScriptRoot\..\..")
$files = @(
    'tests\framework\AetherTest.psm1',
    'tests\cases\explorer_inline_input.tests.ps1',
    'tests\run_tests.ps1',
    'tests\tools\coverage.ps1',
    'tests\tools\capture_window.ps1'
)
$bad = 0
foreach ($f in $files) {
    $t = $null; $e = $null
    [System.Management.Automation.Language.Parser]::ParseFile((Resolve-Path $f), [ref]$t, [ref]$e) | Out-Null
    if ($e) {
        $bad++
        Write-Host "[FAIL] $f"
        $e | ForEach-Object { Write-Host ("  " + $_.Message + " @L" + $_.Extent.StartLineNumber) }
    } else {
        Write-Host "[OK] $f"
    }
}
# 模块可加载性 + 导出函数完整性
Import-Module "$PSScriptRoot\..\framework\AetherTest.psm1" -Force
$fns = (Get-Command -Module AetherTest).Name
Write-Host "模块导出函数: $($fns -join ', ')"
if ($fns.Count -lt 10) { $bad++; Write-Host "[FAIL] 模块导出函数不完整" }
exit $bad
