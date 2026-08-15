# mcp-servers — IDE 附带 MCP 工具集

> 本目录是 **Aether Studio 仓库内附带的 MCP 工具服务**（Python/stdio），
> 与 IDE 本体（`crates/`）同仓但互不依赖：IDE 不调用它们，它们也不依赖 IDE 代码。

## 目录结构

```
mcp-servers/
├── unified/                  # 统一 MCP 入口（单一 server.py，聚合 72 工具）
├── code-analysis-enhance/    # 代码分析增强（13 工具：file_dedup/change_impact/
│                             #   lesson_recall/code_context/lsp_query/aether_*）
├── pr-oracle/                # PR→测试影响分析（3 工具，静态零费用）
├── ci-optimization/          # 纯函数工具集（52 工具：数学/字符串/排序/JSON）
├── tautest/                  # 变异测试（4 工具）
└── browser-use/              # 浏览器自动化
```

每个子模块**相互独立、可单独运行**：

```bash
python <dir>/server.py            # stdio MCP 服务
python <dir>/server.py --selftest # 自检
python -m pytest <dir>/ -q        # 子模块测试（同名 test_*.py 需逐目录跑）
```

## 与 unified-rx-mcp 的关系（重要）

本目录与独立的 [`unified-rx-mcp`](https://github.com/bfxh/unified-rx-mcp) 仓库
是**两套相互独立、互不重叠**的工具集：

| | 本目录 `mcp-servers/` | `unified-rx-mcp` 仓库 |
|---|---|---|
| 定位 | 编辑器周边能力 | 通用 RX MCP |
| 工具 | 72 工具（代码分析/PR 映射/变异测试/数学） | 61 工具（挖漏洞/仓库认知/UI 检查/设计系统） |
| 代码 | Python，随 IDE 仓库维护 | Python，独立仓库维护 |
| 关系 | 无共享代码、无相互依赖 | 无共享代码、无相互依赖 |

迁移工具时请先确认目标仓库归属，避免两套工具集混用。

## 验证状态

- unified 10/10 测试通过
- 子模块回归 112 passed / 1 skipped（unified 10 + code-analysis-enhance 54 +
  pr-oracle 21 + ci-optimization 16 + tautest 11）
- CI（`.github/workflows/ci.yml` 的 `mcp-servers-test` job）每次 push/PR 自动跑
