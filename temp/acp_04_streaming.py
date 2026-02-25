#!/usr/bin/env python3
"""ACP Demo 4: Streaming output — show each chunk as it arrives with timing."""

import json
import subprocess
import sys
import time

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

# Initialize
send(0, "initialize", {
    "protocolVersion": 1,
    "clientCapabilities": {},
    "clientInfo": {"name": "acp-demo", "version": "0.1.0"},
})

# Read init response
while True:
    line = proc.stdout.readline().strip()
    if not line:
        continue
    msg = json.loads(line)
    if msg.get("id") == 0:
        print(f"<<< initialized")
        break

# New session
send(1, "session/new", {"cwd": CWD, "mcpServers": []})
while True:
    line = proc.stdout.readline().strip()
    if not line:
        continue
    msg = json.loads(line)
    if msg.get("id") == 1:
        session_id = msg.get("result", {}).get("sessionId", "")
        print(f"Session: {session_id}")
        break

# Prompt with streaming observation
send(2, "session/prompt", {
    "sessionId": session_id,
    "prompt": [{"type": "text", "text": "hi"}],
})

t0 = time.time()
chunk_count = 0
total_chars = 0

print("\n--- Streaming output ---")
while True:
    line = proc.stdout.readline().strip()
    if not line:
        continue
    msg = json.loads(line)

    # Final response
    if msg.get("id") == 2:
        elapsed = time.time() - t0
        print(f"\n--- Stream complete ---")
        print(f"Chunks: {chunk_count}, Chars: {total_chars}, Time: {elapsed:.2f}s")
        print(f"Stop reason: {msg.get('result', {}).get('stopReason', '')}")
        break

    # Notifications
    if "id" not in msg and msg.get("method") == "session/update":
        u = msg["params"]["update"]
        ut = u.get("sessionUpdate", "")
        if ut == "agent_message_chunk":
            text = u.get("content", {}).get("text", "")
            chunk_count += 1
            total_chars += len(text)
            elapsed = time.time() - t0
            sys.stdout.write(text)
            sys.stdout.flush()
        elif ut == "turn_end":
            print(f"\n[turn_end at {time.time() - t0:.2f}s]")

proc.terminate()
