"""安全修复测试：read_file 排除 + 危险参数限位（security sa_20260809_102048）。"""
import sys
import os

sys.path.insert(0, os.path.dirname(__file__))
import server  # noqa: E402


def test_read_file_not_exposed():
    """read_file 必须被排除（任意文件读取漏洞修复）。"""
    names = [n for n in server.TOOLS if "read_file" in n]
    assert names == [], f"read_file 仍暴露: {names}"


def test_factorial_limit():
    """factorial 超上限被拒（防无界 DoS）。"""
    try:
        server._call("ciopt_factorial_factorial", {"n": 1_000_000})
        assert False, "应拒绝超上限 factorial"
    except ValueError as e:
        assert "上限" in str(e)


def test_power_limit():
    """power 超上限被拒。"""
    try:
        server._call("ciopt_math_operations_power", {"base": 2, "exponent": 10**9})
        assert False, "应拒绝超上限 power"
    except ValueError as e:
        assert "上限" in str(e)


def test_matrix_size_limit():
    """矩阵超 200×200 被拒（防 O(n³) DoS）。"""
    big = [[0] * 300 for _ in range(300)]
    try:
        server._call("ciopt_matrix_matrix_multiplication", {"a": big, "b": big})
        assert False, "应拒绝超上限矩阵"
    except ValueError as e:
        assert "200×200" in str(e)


def test_limits_do_not_block_normal():
    """正常值不受限。"""
    r = server._call("ciopt_factorial_factorial", {"n": 10})
    assert r == "3628800"
    r2 = server._call("ciopt_math_operations_power", {"base": 2, "exponent": 10})
    assert r2 == "1024"
