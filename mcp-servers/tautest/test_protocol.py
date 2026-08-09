"""End-to-end MCP protocol test for the tautest runner server."""

import os
import sys

import pytest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

SERVER = os.path.join(os.path.dirname(os.path.abspath(__file__)), "server.py")
EXAMPLE = r"E:\共享\51\10\tautest\examples\jest-basic"


@pytest.mark.asyncio
async def test_protocol_list_and_call():
    params = StdioServerParameters(command=sys.executable, args=[SERVER])
    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()
            tools = await session.list_tools()
            names = {t.name for t in tools.tools}
            assert names == {"tautest_doctor", "tautest_init", "tautest_run", "tautest_demo"}

            res = await session.call_tool("tautest_doctor", {"repo_path": EXAMPLE})
            assert "Error" not in res.content[0].text
