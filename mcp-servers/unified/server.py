"""Unified MCP server — 单一入口，聚合 4 个独立子模块。

每个子模块保持原样、可独立运行（`python server.py`）：

  module                     tools   prefix        external deps
  code-analysis-enhance       13     (none)        none（lsp_query 需本地 LSP 二进制）
  pr-oracle                    3     pr_oracle_*   pr-test-oracle src + httpx
  ci-optimization             52     ciopt_*       CI-Optimization src（动态发现；实测 52，非 53）
  tautest                      4     tautest_*     node + tautest CLI

聚合设计：
  - importlib 按文件路径加载子模块（4 个文件都叫 server.py，普通 import 必冲突）
  - 注册表：tool name -> (module label, call kind, Tool 定义)
  - fail-fast：启动期发现工具名冲突直接抛错，绝不静默覆盖
  - 错误隔离：单个模块加载失败仅告警跳过；单个工具调用失败转结构化错误文本，
    绝不拖垮网关；未知工具返回明确错误而非 500
  - 统一契约：所有工具返回 list[types.TextContent]（子模块各自的 str 由网关包装）
  - 工具名全部保留原名（含 code-analysis-enhance 的 13 个无前缀工具），
    迁移零成本；新工具建议统一加前缀

运行:  python server.py            (stdio transport)
自检:  python server.py --selftest
测试:  python -m pytest test_unified.py -v
"""

import argparse
import asyncio
import importlib.util
import os
import sys
import traceback

import mcp.server.stdio
import mcp.types as types
from mcp.server import NotificationOptions, Server
from mcp.server.models import InitializationOptions

_BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# (label, 目录名, call kind)
#   "decorator": 模块用 @app.list_tools()/@app.call_tool() 装饰器注册
#   "pure":      模块暴露同步纯函数 _tool_definitions()/_call()
_SUB_SERVERS: list[tuple[str, str, str]] = [
    ("code-analysis-enhance", "code-analysis-enhance", "decorator"),
    ("pr-oracle", "pr-oracle", "pure"),
    ("ci-optimization", "ci-optimization", "pure"),
    ("tautest", "tautest", "pure"),
]

_TOOLS: dict[str, tuple[str, str, types.Tool]] = {}  # name -> (label, kind, def)
_MODULES: dict[str, object] = {}                     # label -> loaded module


def _load_module(label: str, dir_name: str) -> object | None:
    """按文件路径加载子模块；失败返回 None（加载期错误隔离，绝不抛出）。"""
    path = os.path.join(_BASE, dir_name, "server.py")
    try:
        spec = importlib.util.spec_from_file_location(f"unified_{label}", path)
        if spec is None or spec.loader is None:
            return None
        module = importlib.util.module_from_spec(spec)
        sys.modules[spec.name] = module
        spec.loader.exec_module(module)
        return module
    except Exception as exc:
        print(
            f"[unified] WARNING: failed to load module '{label}': "
            f"{type(exc).__name__}: {exc}",
            file=sys.stderr,
        )
        traceback.print_exc(file=sys.stderr)
        return None


async def _call_module(module: object, kind: str, name: str, arguments: dict) -> list[types.TextContent]:
    """统一调用契约：任何子模块都返回 list[TextContent]。"""
    if kind == "decorator":
        # code-analysis-enhance 的 call_tool 本身返回 list[TextContent]
        return await module.call_tool(name, arguments)
    # pure 模块的 _call 返回 str，网关包一层
    text = module._call(name, arguments)
    return [types.TextContent(type="text", text=text)]


def _build_registry() -> None:
    """加载全部子模块并合并工具定义；工具名冲突即抛 RuntimeError（fail-fast）。"""
    _TOOLS.clear()
    _MODULES.clear()
    for label, dir_name, kind in _SUB_SERVERS:
        module = _load_module(label, dir_name)
        if module is None:
            continue
        _MODULES[label] = module
        if kind == "decorator":
            definitions = asyncio.run(module.list_tools())
        else:
            definitions = module._tool_definitions()
        for tool in definitions:
            if tool.name in _TOOLS:
                prev_label, _, _ = _TOOLS[tool.name]
                raise RuntimeError(
                    f"tool name collision: {tool.name!r} declared by both "
                    f"'{prev_label}' and '{label}'"
                )
            _TOOLS[tool.name] = (label, kind, tool)


_build_registry()

app = Server("unified-dev-tools")


@app.list_tools()
async def list_tools() -> list[types.Tool]:
    return [entry[2] for entry in sorted(_TOOLS.values(), key=lambda e: e[2].name)]


@app.call_tool()
async def call_tool(name: str, arguments: dict) -> list[types.TextContent]:
    entry = _TOOLS.get(name)
    if entry is None:
        return [types.TextContent(type="text", text=f"Error: unknown tool: {name}")]
    label, kind, _ = entry
    module = _MODULES.get(label)
    if module is None:
        return [types.TextContent(type="text", text=f"Error: module '{label}' not loaded")]
    try:
        return await _call_module(module, kind, name, arguments or {})
    except Exception as exc:
        # 错误隔离：单个工具失败绝不拖垮网关，转结构化错误文本
        return [
            types.TextContent(
                type="text",
                text=f"Error in {label}.{name}: {type(exc).__name__}: {exc}",
            )
        ]


async def main() -> None:
    async with mcp.server.stdio.stdio_server() as (read_stream, write_stream):
        await app.run(
            read_stream,
            write_stream,
            InitializationOptions(
                server_name="unified-dev-tools",
                server_version="1.0.0",
                capabilities=app.get_capabilities(
                    notification_options=NotificationOptions(),
                    experimental_capabilities={},
                ),
            ),
        )


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--selftest", action="store_true", help="构建注册表并断言工具数后退出")
    args, _ = parser.parse_known_args()
    if args.selftest:
        counts: dict[str, int] = {}
        for name, (label, _, _) in _TOOLS.items():
            counts[label] = counts.get(label, 0) + 1
        for label in sorted(counts):
            print(f"{label}: {counts[label]} tools")
        print(f"total: {len(_TOOLS)} tools")
        assert len(_TOOLS) == 72, f"expected 72 tools, got {len(_TOOLS)}"
        print("self-test passed")
    else:
        asyncio.run(main())
