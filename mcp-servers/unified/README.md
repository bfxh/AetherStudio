# Unified MCP Gateway — 4 个子模块的单一入口

把 4 个独立 Python MCP server 聚合成一个 stdio 入口，子模块保持原样、可独立运行。

| 子模块 | 工具数 | 工具名前缀 | 注册方式 | 外部依赖 |
|---|---|---|---|---|
| code-analysis-enhance | 13 | 无前缀 | `@app.list_tools()` / `@app.call_tool()` 装饰器 | 无网络（lsp_query 需本地 LSP 二进制） |
| pr-oracle | 3 | `pr_oracle_*` | 纯函数 `_tool_definitions()` / `_call()` | pr-test-oracle src + httpx |
| ci-optimization | 52 | `ciopt_*` | 纯函数 + import 时动态发现 | CI-Optimization src |
| tautest | 4 | `tautest_*` | 纯函数 | node + tautest CLI |

合计 **72** 个工具（注：ci-optimization 实测 52，非任务描述中的 53；独立加载验证一致）。

## 使用

```bash
python server.py              # stdio 入口
python server.py --selftest   # 构建注册表并断言工具数
python -m pytest test_unified.py -v
```

客户端接入：见 `.mcp.json`（server 名 `unified-dev-tools`）。

## 聚合设计（最佳实践）

1. **按路径加载，不 import**：4 个文件都叫 `server.py`，普通 import 必冲突；
   `importlib.util.spec_from_file_location` 以 `unified_<label>` 命名加载。
2. **注册表 + fail-fast 冲突检测**：`_TOOLS: name -> (label, kind, Tool)`，
   启动期发现重名直接 `RuntimeError`，绝不静默覆盖。
3. **错误隔离（双层）**：
   - 加载期：单个模块 import 失败仅 stderr 告警并跳过，网关照常启动；
   - 调用期：单个工具异常转 `Error in <label>.<name>: ...` 结构化文本，不崩网关；
   - 未知工具返回明确 `Error: unknown tool`。
4. **统一契约**：子模块各自返回 `str` 或 `list[TextContent]`，网关统一包装为
   `list[TextContent]`，客户端无感知。
5. **命名空间**：保留全部原名（零迁移成本）；新工具建议沿用模块前缀
   （`pr_oracle_*` / `ciopt_*` / `tautest_*`，cae 建议 `cae_*`），冲突检测兜底。

## 测试策略（三层）

1. **注册表层**（`test_total_tool_count` / `test_per_module_counts` /
   `test_no_name_collisions` / `test_prefix_groups` / `test_every_tool_schema_valid` /
   `test_load_missing_module_is_isolated`）：工具总数、分模块计数、名字唯一、
   前缀分组、schema 合法性、加载容错。
2. **协议层 E2E**（`test_protocol_list_all_tools` / `test_protocol_sample_call_each_module`）：
   `ClientSession` 连接网关 → initialize → list_tools 全量 → 每模块抽样调用
   （aether_goto_parse / ciopt_math_operations_add / pr_oracle_map_local / tautest_demo）。
3. **错误隔离层**（`test_unknown_tool_returns_error_not_crash` /
   `test_module_error_does_not_affect_other_tools`）：未知工具不崩、同会话内
   一个模块失败后其它模块仍正常。

回归：子模块各自 `python -m pytest -q`（须逐目录跑——同名 test_server.py
从根目录收集会冲突，与聚合器按路径加载同理）。
