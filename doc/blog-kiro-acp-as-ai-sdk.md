# Build AI-Powered Applications Using Kiro CLI as Your AI Backend

*Use the Agent Client Protocol to build AI apps — no API keys, no SDKs, just one CLI.*

---

## The Problem: Building AI Apps Is Harder Than It Should Be

Every AI-powered application today faces the same setup overhead: obtain API keys, pick an SDK, manage token billing, handle model versioning, and wire up streaming. Before you write a single line of business logic, you're deep in infrastructure plumbing.

What if you could skip all of that and talk to a fully-featured AI agent through a simple subprocess?

## Introducing ACP Support in Kiro CLI

Kiro CLI now implements the [Agent Client Protocol (ACP)](https://agentclientprotocol.com/) — an open standard that defines how clients communicate with AI agents over JSON-RPC 2.0.

ACP was designed to standardize agent-editor communication, much like the [Language Server Protocol (LSP)](https://microsoft.github.io/language-server-protocol/) did for language servers. But the protocol isn't limited to editors — any application that can spawn a process and read/write stdio can be an ACP client.

This means Kiro CLI can serve as a general-purpose AI backend for:

- Desktop applications
- CLI tools and automation scripts
- Editor plugins (JetBrains, Zed, and more)
- Custom workflows in any programming language

No API keys. No SDK dependencies. No token management. Just `kiro-cli acp`.

## How It Works

ACP uses JSON-RPC 2.0 over stdio. Your application spawns `kiro-cli acp` as a child process, sends JSON requests to stdin, and reads JSON responses from stdout. Streaming is built in — the agent sends incremental updates as notifications before the final response.

### Spawning the Agent

```rust
// Rust
let child = Command::new("kiro-cli")
    .arg("acp")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;
```

```python
# Python
import subprocess
proc = subprocess.Popen(
    ["kiro-cli", "acp"],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True
)
```

```javascript
// Node.js
const { spawn } = require("child_process");
const agent = spawn("kiro-cli", ["acp"], { stdio: ["pipe", "pipe", "pipe"] });
```

### Protocol Flow

The lifecycle is straightforward:

```
Client                          kiro-cli acp
  │                                  │
  │─── initialize ──────────────────>│
  │<── capabilities ─────────────────│
  │                                  │
  │─── session/new ─────────────────>│
  │<── { sessionId } ───────────────│
  │                                  │
  │─── session/prompt ──────────────>│
  │<── session/update (chunk) ──────│  ← streaming
  │<── session/update (chunk) ──────│
  │<── session/update (turn_end) ───│
  │<── response ────────────────────│
  │                                  │
  │─── session/set_model ───────────>│  ← optional
  │─── session/cancel ──────────────>│  ← optional
  │─── session/load ────────────────>│  ← restore session
```

### Key Methods

**Initialize** — Handshake to exchange capabilities:

```json
{"jsonrpc":"2.0","id":0,"method":"initialize","params":{
  "protocolVersion":1,
  "clientCapabilities":{},
  "clientInfo":{"name":"my-app","version":"0.1.0"}
}}
```

**Create Session** — Start a new conversation:

```json
{"jsonrpc":"2.0","id":1,"method":"session/new","params":{
  "cwd":"/path/to/project",
  "mcpServers":[]
}}
```

**Send Prompt** — Ask a question and receive streaming responses:

```json
{"jsonrpc":"2.0","id":2,"method":"session/prompt","params":{
  "sessionId":"uuid-here",
  "prompt":[{"type":"text","text":"Explain this code"}]
}}
```

**Switch Model** — Change models mid-session:

```json
{"jsonrpc":"2.0","id":3,"method":"session/set_model","params":{
  "sessionId":"uuid-here",
  "modelId":"claude-sonnet-4"
}}
```

Available models include `auto`, `claude-sonnet-4`, `claude-sonnet-4.5`, `claude-sonnet-4.6`, `claude-opus-4.5`, `claude-opus-4.6`, and `claude-haiku-4.5`.

**Load Session** — Sessions persist at `~/.kiro/sessions/cli/` and can be restored in a new process:

```json
{"jsonrpc":"2.0","id":1,"method":"session/load","params":{
  "sessionId":"uuid-from-before",
  "cwd":"/path/to/project"
}}
```

## What You Get for Free

By using Kiro CLI as your AI backend, your application inherits:

| Capability | Description |
|---|---|
| Streaming | Real-time token-by-token responses via session updates |
| Tool use | The agent can invoke tools (file operations, terminal, etc.) |
| Session persistence | Save and restore conversations across process restarts |
| Model switching | Change models on the fly without reconnecting |
| MCP integration | Pass MCP servers to sessions for extended tool capabilities |
| Cancellation | Interrupt generation mid-stream |

All of this comes from the protocol — your application just needs to handle JSON-RPC messages.

## Example: KiroNotebook

To demonstrate this approach, we built [KiroNotebook](https://github.com/vokako/kiro-notebook) — a local NotebookLM-style application where you can chat with AI about your documents without uploading anything to the cloud.

### What It Does

- **Three-panel layout** — File tree, document preview, and AI chat side by side
- **Document support** — PDF, DOCX, Markdown, TXT, HTML
- **Per-session processes** — Each chat tab spawns its own `kiro-cli acp` instance
- **Context tracking** — Files sent to the agent are marked with ✓; new files are queued until the next message
- **Session persistence** — Close and reopen with full conversation context restored
- **Streaming with cancellation** — Real-time responses that can be interrupted

### Architecture

```
┌─────────────┐      ┌──────────────┐      ┌─────────────────┐
│   React UI  │─────>│ Tauri / Rust │─────>│  kiro-cli acp   │
│             │<─────│              │<─────│  (per session)   │
└─────────────┘      └──────────────┘      └─────────────────┘
    Tauri events        JSON-RPC stdio
```

The Rust backend manages ACP process lifecycles — spawning, message routing, and cleanup. The React frontend handles rendering and user interaction. The entire AI capability comes from Kiro CLI; there are no other AI dependencies.

### Try It

```bash
# Prerequisites: Kiro CLI (authenticated), Node.js 18+, Rust
git clone https://github.com/vokako/kiro-notebook.git
cd kiro-notebook
npm install
npm run tauri dev
```

### Python Examples

The repository also includes standalone Python scripts for testing each ACP method:

```bash
uv run acp-python-example/acp_01_new_session.py   # Create session + prompt
uv run acp-python-example/acp_02_load_session.py   # Load previous session
uv run acp-python-example/acp_03_set_model.py      # Switch model
uv run acp-python-example/acp_04_streaming.py      # Streaming with timing
```

These scripts serve as minimal reference implementations — useful starting points for building your own ACP client in Python.

## Getting Started with Your Own App

Building an ACP client is straightforward in any language:

1. **Spawn** `kiro-cli acp` as a child process
2. **Send** `initialize` to handshake
3. **Create** a session with `session/new`
4. **Prompt** with `session/prompt` and handle streaming updates
5. **Optionally** switch models, cancel generation, or persist sessions

The full ACP specification is available at [agentclientprotocol.com](https://agentclientprotocol.com/), and Kiro's implementation details are documented at [kiro.dev/docs/cli/acp](https://kiro.dev/docs/cli/acp/).

## Conclusion

Kiro CLI's ACP support turns a CLI tool into a programmable AI backend. Instead of integrating SDKs and managing credentials, you spawn a process and speak JSON-RPC. This works for quick prototypes, production desktop apps, editor plugins, or anything in between.

KiroNotebook shows what's possible — a full-featured document chat application with zero AI infrastructure code. The same pattern applies to whatever you want to build.

Give it a try: install [Kiro CLI](https://kiro.dev/downloads/), run `kiro-cli acp`, and start building.

## Resources

- [Kiro CLI ACP Documentation](https://kiro.dev/docs/cli/acp/)
- [Agent Client Protocol Specification](https://agentclientprotocol.com/)
- [KiroNotebook Source Code](https://github.com/vokako/kiro-notebook)
- [Kiro CLI Downloads](https://kiro.dev/downloads/)
