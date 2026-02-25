#!/usr/bin/env python3
"""ACP Demo 2: Load a previous session (from test 1) in a NEW process."""

import json
import subprocess
import sys
import os

KIRO = "/Applications/Kiro CLI.app/Contents/MacOS/kiro-cli"
CWD = "/Users/clawd/work/poc/260225_kiro_notebook"

# Read session ID from test 1
sid_file = "/tmp/acp_test_session_id.txt"
if not os.path.exists(sid_file):
    print("Run acp_01_new_session.py first to create a session.")
    sys.exit(1)
session_id = open(sid_file).read().strip()
print(f"Loading session: {session_id}")

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
                    print(f"[update: {ut}]")
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

# 2. Load session
send(1, "session/load", {
    "sessionId": session_id,
    "cwd": CWD,
    "mcpServers": [],
})
resp = read_until_response(1)

if "error" in resp:
    print("\n*** session/load FAILED — cannot restore session in new process ***")
    proc.terminate()
    sys.exit(1)

loaded_id = resp.get("result", {}).get("sessionId", session_id)
print(f"Loaded session ID: {loaded_id}")

# 3. Follow-up prompt
send(2, "session/prompt", {
    "sessionId": loaded_id,
    "prompt": [{"type": "text", "text": "What was my first message to you?"}],
})
resp = read_until_response(2)
print(f"Stop reason: {resp.get('result', {}).get('stopReason', '')}")

proc.terminate()
