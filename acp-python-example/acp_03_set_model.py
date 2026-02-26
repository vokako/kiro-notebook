#!/usr/bin/env python3
"""ACP Demo 3: Set model for a session."""

import json
import subprocess
import sys

KIRO = "/Applications/Kiro CLI.app/Contents/MacOS/kiro-cli"
CWD = "/Users/clawd/work/poc/260225_kiro_notebook"

def jsonrpc(id, method, params):
    return json.dumps({"jsonrpc": "2.0", "id": id, "method": method, "params": params})

proc = subprocess.Popen(
    [KIRO, "acp"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=sys.stderr, text=True, bufsize=1
)

def send(id, method, params):
    print(f">>> {method} (id={id})")
    proc.stdin.write(jsonrpc(id, method, params) + "\n")
    proc.stdin.flush()

def read_until_response(expected_id):
    while True:
        line = proc.stdout.readline().strip()
        if not line:
            continue
        msg = json.loads(line)
        if "id" not in msg:
            m = msg.get("method", "")
            if m == "session/update":
                u = msg["params"]["update"]
                ut = u.get("sessionUpdate", "")
                if ut == "agent_message_chunk":
                    print(u.get("content", {}).get("text", ""), end="")
                elif ut == "turn_end":
                    print("\n[turn_end]")
            else:
                print(f"[notification] {m}")
            continue
        if msg["id"] == expected_id:
            if "error" in msg:
                print(f"<<< ERROR: {json.dumps(msg['error'])}")
            else:
                print(f"<<< OK: {json.dumps(msg.get('result', {}))[:200]}")
            return msg

# 1. Initialize
send(0, "initialize", {
    "protocolVersion": 1,
    "clientCapabilities": {},
    "clientInfo": {"name": "acp-demo", "version": "0.1.0"},
})
read_until_response(0)

# 2. New session
send(1, "session/new", {"cwd": CWD, "mcpServers": []})
resp = read_until_response(1)
session_id = resp.get("result", {}).get("sessionId", "")
print(f"Session: {session_id}")

# 3. Set model
send(2, "session/set_model", {
    "sessionId": session_id,
    "modelId": "claude-sonnet-4",
})
resp = read_until_response(2)
print(f"set_model result: {json.dumps(resp.get('result', resp.get('error', {})))}")

# 4. Prompt to verify
send(3, "session/prompt", {
    "sessionId": session_id,
    "prompt": [{"type": "text", "text": "What model are you? Reply in one line."}],
})
resp = read_until_response(3)
print(f"Stop reason: {resp.get('result', {}).get('stopReason', '')}")

proc.terminate()
