"""Tests for the pr-oracle static MCP server (local analysis only; no network)."""

import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import server  # noqa: E402

SAMPLE_REPO = r"E:\共享\51\10\CI-Optimization"
PATTERNS = ["**/test_*.py"]


def test_discover_tests():
    out = server.discover_tests(SAMPLE_REPO, PATTERNS)
    data = json.loads(out)
    assert data["repo"] == SAMPLE_REPO
    assert len(data["test_files"]) >= 20, f"expected >=20 tests, got {len(data['test_files'])}"


def test_map_local_finds_candidate():
    out = server.map_local(SAMPLE_REPO, ["src/math_operations.py"], PATTERNS)
    data = json.loads(out)
    assert data["test_files_discovered"] >= 20
    mapping = next(m for m in data["mappings"] if m["source_file"] == "src/math_operations.py")
    assert any("test_math_operations" in t for t in mapping["candidate_tests"]), mapping


def test_map_local_config_broad_impact():
    out = server.map_local(SAMPLE_REPO, ["requirements.txt"], PATTERNS)
    data = json.loads(out)
    mapping = next(m for m in data["mappings"] if m["source_file"] == "requirements.txt")
    assert "Config file" in mapping["mapping_reason"]
    assert len(mapping["candidate_tests"]) >= 20


def test_map_local_bad_path():
    out = server.map_local(r"Z:\nonexistent", ["a.py"], PATTERNS)
    assert "Error" in out


def test_unknown_tool():
    try:
        server._call("pr_oracle_nope", {})
        raise AssertionError("expected ValueError")
    except ValueError:
        pass
