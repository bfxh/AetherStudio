"""Verify browser-use MCP servers start and expose tools over stdio."""

import sys

import pytest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client


async def _list_tools(module: str) -> list[str]:
    params = StdioServerParameters(command=sys.executable, args=["-m", module])
    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            tools = await session.list_tools()
            return sorted(t.name for t in tools.tools)


@pytest.mark.asyncio
async def test_full_mcp_server_lists_tools():
    names = await _list_tools("browser_use.mcp")
    assert len(names) > 0
    print("full MCP tools:", names)


@pytest.mark.asyncio
async def test_cli_mcp_server_lists_tools():
    names = await _list_tools("browser_use.mcp.cli_mcp")
    assert names == ["browser_exec", "browser_screenshot"]
    print("cli MCP tools:", names)
