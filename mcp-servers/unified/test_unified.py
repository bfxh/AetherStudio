"""Tests for the unified MCP gateway (4 sub-servers merged into one entry).

覆盖三层：
  1. 注册表层：工具总数、名字唯一（冲突检测）、前缀分组、schema 合法性、加载容错
  2. 协议层：ClientSession E2E —— list_tools 全量 + 每模块抽样调用
  3. 错误隔离层：未知工具 / 模块失败不影响其它工具（同一会话内连续调用验证）
"""

import os
import sys

import pytest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import server as unified  # noqa: E402

from mcp import ClientSession, StdioServerParameters  # noqa: E402
from mcp.client.stdio import stdio_client  # noqa: E402

SERVER_PATH = os.path.join(os.path.dirname(os.path.abspath(__file__)), "server.py")

# 与子模块独立运行时的工具数完全一致（13 + 3 + 52 + 4；
# 注：ci-optimization 实测为 52，而非任务描述中的 53——独立加载验证一致）
EXPECTED_COUNTS = {
    "code-analysis-enhance": 13,
    "pr-oracle": 3,
    "ci-optimization": 52,
    "tautest": 4,
}
EXPECTED_TOTAL = sum(EXPECTED_COUNTS.values())


def _all_tools():
    return list(unified._TOOLS.values())


# ───────────────────────── 1. 注册表层 ─────────────────────────


def test_total_tool_count():
    assert len(unified._TOOLS) == EXPECTED_TOTAL


def test_per_module_counts():
    counts: dict[str, int] = {}
    for name, (label, _, _) in unified._TOOLS.items():
        counts[label] = counts.get(label, 0) + 1
    assert counts == EXPECTED_COUNTS


def test_no_name_collisions():
    names = [t.name for _, _, t in _all_tools()]
    assert len(names) == len(set(names)), "tool names must be globally unique"


def test_prefix_groups():
    names = {t.name for _, _, t in unified._TOOLS.values()}
    ciopt = [n for n in names if n.startswith("ciopt_")]
    pr = [n for n in names if n.startswith("pr_oracle_")]
    ta = [n for n in names if n.startswith("tautest_")]
    plain = [n for n in names if not n.startswith(("ciopt_", "pr_oracle_", "tautest_"))]
    assert len(ciopt) >= 50, f"ciopt_* expected >=50, got {len(ciopt)}"
    assert len(pr) == 3, f"pr_oracle_* expected 3, got {len(pr)}"
    assert len(ta) == 4, f"tautest_* expected 4, got {len(ta)}"
    # code-analysis-enhance 的 13 个无前缀工具全部保留原名
    assert len(plain) == 13, f"unprefixed expected 13, got {len(plain)}"


def test_every_tool_schema_valid():
    for name, (label, _, tool) in unified._TOOLS.items():
        schema = tool.inputSchema
        assert schema.get("type") == "object", f"{name} schema must be object"
        assert isinstance(schema.get("properties"), dict), f"{name} needs properties"
        # required 可选（cae 的 aether_model_provider/aether_probe 无必填参数）
        assert "required" not in schema or isinstance(schema["required"], list), f"{name} required must be a list"


def test_load_missing_module_is_isolated():
    # 加载不存在的子模块返回 None 而不抛异常（加载期错误隔离）
    assert unified._load_module("nope", "nope-dir") is None


# ───────────────────────── 2. 协议层 E2E ─────────────────────────


@pytest.mark.asyncio
async def test_protocol_list_all_tools():
    params = StdioServerParameters(command=sys.executable, args=[SERVER_PATH])
    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            tools = await session.list_tools()
            names = {t.name for t in tools.tools}
            assert len(names) == EXPECTED_TOTAL
            assert names == set(unified._TOOLS.keys())


@pytest.mark.asyncio
async def test_protocol_sample_call_each_module():
    params = StdioServerParameters(command=sys.executable, args=[SERVER_PATH])
    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            await session.list_tools()

            # code-analysis-enhance：纯静态定位解析（无外部依赖）
            res = await session.call_tool("aether_goto_parse", {"goto": "src/main.rs:42:7"})
            assert "line" in res.content[0].text and "column" in res.content[0].text

            # ci-optimization：纯函数
            res = await session.call_tool("ciopt_math_operations_add", {"a": 2, "b": 3})
            assert res.content[0].text == "5"

            # pr-oracle：本地仓库静态映射（repo 用 mcp-servers 自身）
            res = await session.call_tool(
                "pr_oracle_map_local",
                {"repo_path": unified._BASE, "changed_files": ["code-analysis-enhance/server.py"]},
            )
            assert '"mappings"' in res.content[0].text

            # tautest：demo 只打印不写盘，repo 路径用 mcp-servers 目录
            res = await session.call_tool("tautest_demo", {"repo_path": unified._BASE})
            assert res.content[0].text.strip(), "tautest demo should return text"


# ───────────────────────── 3. 错误隔离层 ─────────────────────────


@pytest.mark.asyncio
async def test_unknown_tool_returns_error_not_crash():
    params = StdioServerParameters(command=sys.executable, args=[SERVER_PATH])
    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            res = await session.call_tool("no_such_tool_xyz", {})
            assert "Error" in res.content[0].text
            # 网关仍存活
            tools = await session.list_tools()
            assert len(tools.tools) == EXPECTED_TOTAL


@pytest.mark.asyncio
async def test_module_error_does_not_affect_other_tools():
    params = StdioServerParameters(command=sys.executable, args=[SERVER_PATH])
    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            await session.list_tools()

            # 让一个工具失败：tautest_doctor 传不存在的目录
            bad = await session.call_tool("tautest_doctor", {"repo_path": r"Z:\definitely\missing"})
            assert "Error" in bad.content[0].text

            # 同一会话内其它模块工具仍正常
            ok = await session.call_tool("ciopt_math_operations_add", {"a": 2, "b": 3})
            assert ok.content[0].text == "5"
