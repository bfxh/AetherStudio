"""Smoke test for pr_oracle_map_pr against a real public GitHub PR.

Uses octocat/Hello-World PR 1 (tiny, classic test PR). Network required;
skipped gracefully when GitHub is unreachable.
"""

import json
import sys
import os

import pytest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import server  # noqa: E402


@pytest.mark.smoke
def test_map_pr_real_public_pr():
    try:
        out = server.map_pr("https://github.com/octocat/Hello-World/pull/1")
    except Exception as exc:  # network / rate limit -> report as environment issue
        pytest.skip(f"GitHub unreachable: {exc}")
    assert "Error" not in out, out
    data = json.loads(out)
    assert data["pr"] == "https://github.com/octocat/Hello-World/pull/1"
    assert isinstance(data["changed_files"], list)
    assert isinstance(data["mappings"], list)
    assert data["head_branch"]
