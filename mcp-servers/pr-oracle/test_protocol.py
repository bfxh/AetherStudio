"""End-to-end MCP protocol test for the pr-oracle static server."""

import json
import os

import pytest
from mcp import ClientSession, StdioServerParameters
from mcp.client.stdio import stdio_client

SERVER = os.path.join(os.path.dirname(os.path.abspath(__file__)), "server.py")
SAMPLE_REPO = r"E:\共享\51\10\CI-Optimization"


@pytest.mark.asyncio
async def test_protocol_list_and_call():
    params = StdioServerParameters(command=sys_executable(), args=[SERVER])
    async with stdio_client(params) as (read, write):
        async with ClientSession(read, write) as session:
            await session.initialize()

            tools = await session.list_tools()
            names = {t.name for t in tools.tools}
            assert names == {"pr_oracle_map_local", "pr_oracle_discover_tests", "pr_oracle_map_pr"}

            res = await session.call_tool(
                "pr_oracle_discover_tests",
                {"repo_path": SAMPLE_REPO, "test_patterns": ["**/test_*.py"]},
            )
            data = json.loads(res.content[0].text)
            assert len(data["test_files"]) >= 20


def sys_executable() -> str:
    import sys

    return sys.executable
