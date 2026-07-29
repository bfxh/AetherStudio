#!/usr/bin/env python3
"""模拟 Aether 的 LSP 握手：对空文件夹启动 rust-analyzer，捕获 stderr 与退出码"""
import json
import os
import subprocess
import sys
import threading
import time

EMPTY = os.path.join(os.environ["TEMP"], "aether_empty_repro")
URI = "file:///" + EMPTY.replace("\\", "/").replace(":", "%3A", 1).replace("%3A", ":", 1)

def lsp_msg(payload: dict) -> bytes:
    body = json.dumps(payload).encode()
    return f"Content-Length: {len(body)}\r\n\r\n".encode() + body

proc = subprocess.Popen(
    ["rust-analyzer"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    cwd=os.path.dirname(os.path.abspath(__file__)),  # 模拟应用 cwd（项目目录）
)
print(f"rust-analyzer PID={proc.pid}, rootUri={URI}")

stderr_lines = []
def drain_stderr():
    for line in proc.stderr:
        text = line.decode(errors="replace").rstrip()
        stderr_lines.append(text)
        print(f"[stderr] {text}", flush=True)

def drain_stdout():
    data = proc.stdout.read()
    if data:
        print(f"[stdout] {len(data)} bytes: {data[:2000].decode(errors='replace')}", flush=True)

threading.Thread(target=drain_stderr, daemon=True).start()
threading.Thread(target=drain_stdout, daemon=True).start()

# initialize（与 aether-lsp server.rs initialize() 等价的最小参数）
init = {
    "jsonrpc": "2.0", "id": 1, "method": "initialize",
    "params": {
        "processId": os.getpid(),
        "rootUri": URI,
        "workspaceFolders": [{"uri": URI, "name": "aether_empty_repro"}],
        "capabilities": {},
    },
}
proc.stdin.write(lsp_msg(init))
proc.stdin.flush()
time.sleep(2)
proc.stdin.write(lsp_msg({"jsonrpc": "2.0", "method": "initialized", "params": {}}))
proc.stdin.flush()

# 观察至多 60 秒
for i in range(60):
    code = proc.poll()
    if code is not None:
        print(f"\n!!! rust-analyzer 在 {i} 秒后退出, exit code = {code}")
        sys.exit(0)
    time.sleep(1)

print("\n60 秒后 rust-analyzer 仍存活")
proc.kill()
