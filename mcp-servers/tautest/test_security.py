"""tautest 安全修复测试（security sa_20260809_102435）。"""
import sys
import os

sys.path.insert(0, os.path.dirname(__file__))
import server  # noqa: E402


def test_sanitize_allowed():
    """白名单参数通过。"""
    assert server._sanitize_extra_args(["--all"]) == ["--all"]
    assert server._sanitize_extra_args(["--coverage"]) == ["--coverage"]
    assert server._sanitize_extra_args(["--diff", "main"]) == ["--diff", "main"]


def test_sanitize_rejects_output():
    """--output 被拒（任意文件写）。"""
    assert server._sanitize_extra_args(["--output", "C:\\evil.txt"]) is None


def test_sanitize_rejects_unknown():
    """未知参数被拒。"""
    assert server._sanitize_extra_args(["--rm", "-rf"]) is None
    assert server._sanitize_extra_args(["; rm -rf /"]) is None


def test_sanitize_rejects_bad_diff():
    """--diff 后跟非法 ref 被拒（注入）。"""
    assert server._sanitize_extra_args(["--diff", "--output"]) is None
    assert server._sanitize_extra_args(["--diff", "x;rm"]) is None


def test_call_missing_repo_path():
    """缺 repo_path 返回错误而非 KeyError。"""
    out = server._call("tautest_doctor", {})
    assert "repo_path" in out and "Error" in out


def test_call_force_strict_bool(monkeypatch):
    """字符串 'false' 不触发 force（真正验证 args 不含 --force）。"""
    captured = {}

    def fake_run(repo_path, args, timeout):
        captured["args"] = args
        return "ok"

    monkeypatch.setattr(server, "_run_cli", fake_run)
    server._call("tautest_init", {"repo_path": r"C:\tmp", "force": "false"})
    assert "--force" not in captured["args"], f"force 字符串误判为 True: {captured['args']}"
    # 真正 True 时包含 --force
    server._call("tautest_init", {"repo_path": r"C:\tmp", "force": True})
    assert "--force" in captured["args"]
