"""pr-oracle 安全修复测试（security sa_20260809_101856）。"""
import sys
import os

sys.path.insert(0, os.path.dirname(__file__))
import server  # noqa: E402


def test_changed_files_limit():
    """changed_files 超 200 被拒。"""
    out = server._call("pr_oracle_map_local", {
        "repo_path": "C:\\tmp", "changed_files": [f"f{i}.py" for i in range(201)],
    })
    assert "200" in out and "Error" in out


def test_patterns_limit():
    """test_patterns 超 20 被拒。"""
    out = server._call("pr_oracle_discover_tests", {
        "repo_path": "C:\\tmp", "test_patterns": [f"p{i}" for i in range(21)],
    })
    assert "20" in out and "Error" in out


def test_pattern_path_traversal():
    """含 .. 的 glob 模式被拒（路径遍历）。"""
    out = server._call("pr_oracle_discover_tests", {
        "repo_path": "C:\\tmp", "test_patterns": ["../../**/*.py"],
    })
    assert "非法路径" in out


def test_pattern_windows_abs_path():
    """Windows 反斜杠绝对路径被拒（review sa_20260809_103911 修复）。"""
    out = server._call("pr_oracle_discover_tests", {
        "repo_path": "C:\\tmp", "test_patterns": ["\\Windows\\system32\\*.dll"],
    })
    assert "非法路径" in out


def test_pattern_drive_relative_path():
    """盘符相对路径 C:foo.py 被拒（review sa_20260809_104242 盲区修复）。"""
    out = server._call("pr_oracle_discover_tests", {
        "repo_path": "C:\\tmp", "test_patterns": ["C:foo.py"],
    })
    assert "非法路径" in out


def test_pattern_slash_abs_and_unc():
    """正斜杠绝对路径 / 与 UNC 路径被拒。"""
    for pat in ["/etc/passwd", "\\\\server\\share\\*.dll"]:
        out = server._call("pr_oracle_discover_tests", {
            "repo_path": "C:\\tmp", "test_patterns": [pat],
        })
        assert "非法路径" in out, f"应拒绝: {pat}"


def test_head_ref_validation():
    """非法 head_ref 被拒（git 参数注入）。"""
    assert server._validate_head_ref("-h") is False
    assert server._validate_head_ref("feature/x") is True
    assert server._validate_head_ref("../../etc") is False


def test_clone_url_validation():
    """非 github.com 的 clone_url 被拒。"""
    assert server._is_safe_clone_url("https://evil.com/repo.git") is False
    assert server._is_safe_clone_url("https://github.com/a/b.git") is True


def test_repo_limits_normal(tmp_path):
    """正常小仓库通过限制。"""
    (tmp_path / "a.py").write_text("x")
    (tmp_path / "tests").mkdir()
    (tmp_path / "tests" / "test_a.py").write_text("y")
    assert server._check_repo_limits(str(tmp_path)) is None


def test_repo_limits_file_count(tmp_path):
    """文件数超上限被拒（review sa_20260809_105355 补测）。"""
    for i in range(51):
        (tmp_path / f"f{i}.py").write_text("x")
    err = server._check_repo_limits(str(tmp_path), max_files=50)
    assert err and "文件数" in err


def test_repo_limits_size(tmp_path):
    """体积超上限被拒。"""
    (tmp_path / "big.bin").write_bytes(b"x" * 1024)
    err = server._check_repo_limits(str(tmp_path), max_bytes=100)
    assert err and "MB" in err


def test_repo_limits_skips_git(tmp_path):
    """.git 目录不计数。"""
    (tmp_path / ".git").mkdir()
    (tmp_path / ".git" / "objects").mkdir()
    (tmp_path / ".git" / "objects" / "pack").write_bytes(b"x" * 10_000)
    (tmp_path / "a.py").write_text("x")
    assert server._check_repo_limits(str(tmp_path), max_bytes=100) is None


def test_repo_limits_exact_boundary(tmp_path):
    """恰好等于上限应通过（security sa_20260809_105727 补测：锁严格 > 语义）。"""
    for i in range(50):
        (tmp_path / f"f{i}.py").write_text("x")
    assert server._check_repo_limits(str(tmp_path), max_files=50) is None
    # 体积边界用独立目录（避免与文件数测试叠加）
    size_dir = tmp_path / "sizedir"
    size_dir.mkdir()
    (size_dir / "b.bin").write_bytes(b"x" * 50)
    assert server._check_repo_limits(str(size_dir), max_bytes=50) is None


def test_repo_limits_subdir_git(tmp_path):
    """子目录 .git 不计数（security sa_20260809_105727 补测）。"""
    (tmp_path / "sub").mkdir()
    (tmp_path / "sub" / ".git").mkdir()
    (tmp_path / "sub" / ".git" / "pack").write_bytes(b"x" * 10_000)
    (tmp_path / "a.py").write_text("x")
    assert server._check_repo_limits(str(tmp_path), max_bytes=100) is None
