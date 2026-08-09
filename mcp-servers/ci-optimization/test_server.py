"""Tests for the ci-optimization MCP server discovery and call logic."""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import server  # noqa: E402


def test_discovery_count():
    assert len(server.TOOLS) >= 50, f"expected >=50 tools, got {len(server.TOOLS)}"


def test_long_simulations_excluded():
    names = set(server.TOOLS)
    for excluded in (
        "process_large_dataset",
        "perform_database_backup",
        "simulate_file_transfer",
        "generate_detailed_report",
        "render_high_quality_video",
        "write_file",
    ):
        assert not any(excluded in n for n in names), f"{excluded} must be excluded"


def test_call_math_add():
    assert server._call("ciopt_math_operations_add", {"a": 2, "b": 3}) == "5"


def test_call_string_reverse():
    assert server._call("ciopt_string_operations_reverse_string", {"s": "abc"}) == "cba"


def test_call_fibonacci():
    assert server._call("ciopt_fibonacci_fibonacci", {"n": 7}) == "[0, 1, 1, 2, 3, 5, 8]"


def test_call_json_roundtrip():
    out = server._call("ciopt_json_utils_dict_to_json", {"dictionary": {"a": 1}})
    assert '"a"' in out and "1" in out


def test_unknown_tool():
    try:
        server._call("ciopt_nope_missing", {})
        raise AssertionError("expected ValueError")
    except ValueError:
        pass


def test_missing_required_arg():
    try:
        server._call("ciopt_math_operations_add", {"a": 1})
        raise AssertionError("expected ValueError")
    except ValueError as exc:
        assert "b" in str(exc)


def test_default_args_used():
    # functions with defaults must not require the defaulted param
    out = server._call("ciopt_datetime_utils_get_current_time", {})
    assert out


def test_schema_valid_json():
    for name, (mod_name, fn_name, fn, doc) in server.TOOLS.items():
        schema = server._build_schema(fn)
        assert schema["type"] == "object"
        assert "properties" in schema
        assert isinstance(schema["required"], list)
