"""Tests for the tautest MCP server wrapper (uses bundled example repo)."""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import server  # noqa: E402

EXAMPLE = r"E:\共享\51\10\tautest\examples\jest-basic"


def test_doctor_on_example():
    out = server.tautest_doctor(EXAMPLE)
    assert "Error" not in out, out
    assert out.strip(), "expected non-empty doctor output"


def test_demo_prints():
    out = server.tautest_demo(EXAMPLE)
    assert out.strip(), "expected demo output"


def test_bad_path():
    out = server.tautest_doctor(r"Z:\nonexistent")
    assert "Error" in out


def test_unknown_tool():
    try:
        server._call("tautest_nope", {"repo_path": EXAMPLE})
        raise AssertionError("expected ValueError")
    except ValueError:
        pass
