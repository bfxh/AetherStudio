# Unified MCP — 统一开发工具集

本分支将整合的 MCP 工具集作为独立分支纳入 AetherStudio 仓库。

## 内容

- `mcp-servers/unified/` — 统一 MCP 入口（单一 `server.py`，聚合 72 工具）
- `mcp-servers/code-analysis-enhance/` — 代码分析增强（13 工具：file_dedup/change_impact/lesson_recall/code_context/lsp_query/aether_*）
- `mcp-servers/pr-oracle/` — PR→测试影响分析（3 工具，静态零费用）
- `mcp-servers/ci-optimization/` — 纯函数工具集（52 工具：数学/字符串/排序/JSON）
- `mcp-servers/tautest/` — 变异测试（4 工具）

## 使用

```bash
cd mcp-servers/unified
python server.py            # stdio MCP 服务（72 工具单一入口）
python server.py --selftest # 自检
python -m pytest test_unified.py -q
```

子模块各自可独立运行：`python <dir>/server.py` + `python -m pytest <dir>/ -q`

## 验证状态（2026-08-09）

- unified 10/10 测试通过
- 子模块回归 102 passed / 1 skipped（零侵入）

## 本分支另含

- `fix/ai-edit-path-traversal` 的两个安全修复（路径穿越防护，PR #108）
