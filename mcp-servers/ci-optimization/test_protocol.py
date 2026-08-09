"""End-to-end MCP protocol test: spawn server.py over stdio and call tools."""

import os
import sys

import pytest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

SERVER = os.path.join(os.path.dirname(os.path.abspath(__file__)), "server.py")


@pytest.mark.asyncio
async def test_protocol_initialize_list_and_call():
    params = StdioServerParameters(command=sys.executable, args=[SERVER])
    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()

            tools = await session.list_tools()
            assert len(tools.tools) >= 50
            names = {t.name for t in tools.tools}
            assert "ciopt_math_operations_add" in names
            assert "ciopt_write_file" not in names

            res = await session.call_tool("ciopt_math_operations_add", {"a": 2, "b": 3})
            assert "5" in res.content[0].text

            res2 = await session.call_tool(
                "ciopt_string_operations_reverse_string", {"s": "hello"}
            )
            assert "olleh" in res2.content[0].text
