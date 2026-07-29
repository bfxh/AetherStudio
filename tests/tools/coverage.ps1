# 覆盖率工具：插桩测试 + llvm-cov 报告一体化
#（合并自原 run_coverage.ps1 / run_final_coverage.ps1 / generate_coverage_report.ps1）
#
# 用法：
#   pwsh -File tests\tools\coverage.ps1              # 跑插桩测试 + 生成报告
#   pwsh -File tests\tools\coverage.ps1 -ReportOnly  # 只用已有 profraw 生成报告
param([switch]$ReportOnly)

$ErrorActionPreference = "Stop"
$projectRoot = (Resolve-Path "$PSScriptRoot\..\..").Path
Push-Location $projectRoot

try {
    $coverageDir = "tests\coverage"

    if (-not $ReportOnly) {
        # ---- 阶段 1：插桩测试 ----
        if (Test-Path $coverageDir) { Remove-Item -Recurse -Force $coverageDir }
        New-Item -ItemType Directory -Path $coverageDir | Out-Null

        $env:CARGO_INCREMENTAL = "0"
        $env:RUSTFLAGS = "-C instrument-coverage"
        $env:LLVM_PROFILE_FILE = "tests/coverage/%p-%m.profraw"
        cargo test --workspace --no-fail-fast 2>&1 |
            Tee-Object -FilePath "$coverageDir\cargo_test_coverage.log"
    }

    # ---- 阶段 2：合并 profraw 并生成报告 ----
    $toolchainBin = "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib\rustlib\x86_64-pc-windows-msvc\bin"
    $llvmProfdata = "$toolchainBin\llvm-profdata.exe"
    $llvmCov = "$toolchainBin\llvm-cov.exe"
    if (-not (Test-Path $llvmProfdata)) {
        throw "未找到 llvm-profdata，请安装组件: rustup component add llvm-tools"
    }

    $profrawFiles = Get-ChildItem -Path . -Filter *.profraw -Recurse -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty FullName
    if ($profrawFiles.Count -eq 0) { throw "未找到 .profraw 文件，请先跑插桩测试" }
    Write-Host "发现 $($profrawFiles.Count) 个 profraw 文件"

    $mergedProfile = "$coverageDir\merged.profdata"
    & $llvmProfdata merge -sparse @profrawFiles -o $mergedProfile
    if ($LASTEXITCODE -ne 0) { throw "llvm-profdata merge 失败" }

    # 只保留项目 crate 的测试二进制（排除第三方依赖）
    $testBinaries = Get-ChildItem -Path "target\x86_64-pc-windows-msvc\debug\deps" -Filter "*.exe" |
        Where-Object { $_.Name -match '^aether(_|-)' -and $_.Name -notmatch '\.\d+\.exe$' } |
        Select-Object -ExpandProperty FullName
    Write-Host "发现 $($testBinaries.Count) 个测试二进制"

    $commonArgs = @("--instr-profile=$mergedProfile")
    foreach ($bin in $testBinaries) { $commonArgs += "--object=$bin" }
    $commonArgs += "--ignore-filename-regex=(\.cargo|registry|target)"

    & $llvmCov report --use-color=false @commonArgs |
        Tee-Object -FilePath "$coverageDir\coverage_report.txt"
    & $llvmCov export --format=lcov @commonArgs > "$coverageDir\coverage.lcov"

    Write-Host "文本报告: $coverageDir\coverage_report.txt"
    Write-Host "LCOV 报告: $coverageDir\coverage.lcov"
} finally {
    Pop-Location
}
