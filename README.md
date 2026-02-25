<p align="center">
  <img src="icon.png" width="128" height="128" alt="KiroNotebook">
</p>

<h1 align="center">KiroNotebook</h1>

<p align="center">
  A desktop app demonstrating how to build on <a href="https://kiro.dev/docs/cli/acp/">Kiro CLI's Agent Client Protocol (ACP)</a>
</p>

<p align="center">
  Built with <a href="https://v2.tauri.app">Tauri 2</a> · React · TypeScript · Rust
</p>

---

## Why This Project?

[Kiro CLI](https://kiro.dev/cli/) supports the [Agent Client Protocol (ACP)](https://agentclientprotocol.com/) — an open standard that lets any application integrate AI agent capabilities over a simple JSON-RPC 2.0 interface via stdio. This project is a working example of how to build a full-featured ACP client from scratch.

If you're looking to integrate Kiro's AI capabilities into your own editor, tool, or application, this codebase shows you exactly how.

## ACP Integration Guide

### Spawning the ACP Server

Kiro CLI acts as an ACP server. Spawn it as a child process and communicate over stdin/stdout:

```rust
// Rust example (see src-tauri/src/acp.rs)
let mut child = Command::new("kiro-cli")
    .arg("acp")
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;
```

```python
# Python example (see temp/acp_01_new_session.py)
proc = subprocess.Popen(
    ["kiro-cli", "acp"],
    stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True
)
```

### Protocol Flow

Every ACP interaction follows this sequence:

```
Client                              kiro-cli acp
  │                                      │
  │──── initialize ─────────────────────▶│
  │◀─── capabilities ───────────────────│
  │                                      │
  │──── session/new ────────────────────▶│
  │◀─── { sessionId } ─────────────────│
  │                                      │
  │──── session/prompt ─────────────────▶│
  │◀─── session/update (chunk) ─────────│  ← streaming
  │◀─── session/update (chunk) ─────────│  ← streaming
  │◀─── session/update (turn_end) ──────│
  │◀─── response { stopReason } ────────│
  │                                      │
  │──── session/cancel ─────────────────▶│  ← optional: interrupt
  │──── session/set_model ──────────────▶│  ← optional: switch model
```

### Key ACP Methods

#### 1. Initialize — Handshake

```json
{"jsonrpc":"2.0","id":0,"method":"initialize","params":{
  "protocolVersion":1,
  "clientCapabilities":{},
  "clientInfo":{"name":"my-app","version":"0.1.0"}
}}
```

Response includes `agentCapabilities.loadSession: true` — meaning sessions can be persisted and restored.

#### 2. Create a Session

```json
{"jsonrpc":"2.0","id":1,"method":"session/new","params":{
  "cwd":"/path/to/project",
  "mcpServers":[]
}}
// Response: { "sessionId": "uuid-here" }
```

#### 3. Send a Prompt (Streaming)

```json
{"jsonrpc":"2.0","id":2,"method":"session/prompt","params":{
  "sessionId":"uuid-here",
  "prompt":[{"type":"text","text":"Explain this code"}]
}}
```

> ⚠️ The parameter is `prompt`, not `content`.

Before the response arrives, you'll receive streaming notifications:

```json
{"jsonrpc":"2.0","method":"session/update","params":{
  "update":{"sessionUpdate":"agent_message_chunk","content":{"text":"Here's..."}}
}}
```

#### 4. Load a Previous Session

Sessions are persisted at `~/.kiro/sessions/cli/`. You can restore them in a new process:

```json
{"jsonrpc":"2.0","id":1,"method":"session/load","params":{
  "sessionId":"uuid-from-before",
  "cwd":"/path/to/project",
  "mcpServers":[]
}}
```

The agent replays conversation history via `session/update` notifications, then the AI has full context of the previous conversation.

> ⚠️ A session can only be loaded if no other process holds its lock file (`~/.kiro/sessions/cli/<id>.lock`).

#### 5. Switch Model

```json
{"jsonrpc":"2.0","id":3,"method":"session/set_model","params":{
  "sessionId":"uuid-here",
  "modelId":"claude-sonnet-4"
}}
```

> ⚠️ The parameter is `modelId`, not `model`.

Available models: `auto`, `claude-sonnet-4.6`, `claude-opus-4.6`, `claude-sonnet-4.5`, `claude-opus-4.5`, `claude-sonnet-4`, `claude-haiku-4.5`

#### 6. Cancel Generation

```json
{"jsonrpc":"2.0","id":99,"method":"session/cancel","params":{
  "sessionId":"uuid-here"
}}
```

### Gotchas We Discovered

| Issue | Detail |
|-------|--------|
| `prompt` not `content` | `session/prompt` params use `prompt` field, not `content` |
| `modelId` not `model` | `session/set_model` params use `modelId` field |
| Session locking | Each session has a `.lock` file; `session/load` fails if another process holds it |
| Child process cleanup | `kiro-cli` spawns `kiro-cli-chat` as a child; you must kill the entire process group to release the lock |
| Process group kill | On Unix, spawn with `process_group(0)` and kill with `kill(-pid, SIGTERM)` |

### Python Test Scripts

The `temp/` directory contains standalone Python scripts for testing each ACP method:

```bash
# Test 1: Create session + send prompt
uv run temp/acp_01_new_session.py

# Test 2: Load a previous session in a new process
uv run temp/acp_02_load_session.py

# Test 3: Set model
uv run temp/acp_03_set_model.py

# Test 4: Streaming output with timing
uv run temp/acp_04_streaming.py
```

These scripts are useful as reference implementations and for debugging ACP integration.

## The App Itself

KiroNotebook is a local NotebookLM-style app — chat with AI about your documents without uploading anything.

### Features

- **Three-panel layout** — File tree, document preview, AI chat
- **Document support** — PDF, DOCX, Markdown, TXT, HTML
- **Per-session ACP processes** — Each chat tab is an independent `kiro-cli acp` instance
- **Context tracking** — Files sent to AI marked with ✓, new files queued until next message
- **Session persistence** — History saved to `{workspace}/.kiro-notebook/`, restorable with full ACP context
- **Streaming + cancel** — Real-time responses, interruptible mid-generation
- **Model switching** — All Kiro CLI models available from the toolbar

### Architecture

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  React UI   │────▶│   Tauri (Rust)   │────▶│  kiro-cli acp   │
│             │◀────│                  │◀────│  (per session)  │
└─────────────┘     └──────────────────┘     └─────────────────┘
   Tauri events         invoke/commands        JSON-RPC stdio

   • App.tsx            • acp.rs               • initialize
   • App.css            • commands.rs          • session/new
                        • file_reader.rs       • session/prompt
                                               • session/load
                                               • session/cancel
                                               • session/set_model
```

## Getting Started

### Prerequisites

- [Kiro CLI](https://kiro.dev/downloads/) installed and authenticated (`kiro-cli --version`)
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) toolchain

### Run

```bash
git clone https://github.com/vokako/kiro-notebook.git
cd kiro-notebook
npm install
npm run tauri dev
```

### Build

```bash
npm run tauri build -- --bundles app
# Output: src-tauri/target/release/bundle/macos/KiroNotebook.app
```

## Further Reading

- [Kiro CLI ACP Documentation](https://kiro.dev/docs/cli/acp/)
- [ACP Specification](https://agentclientprotocol.com/)
- [Tauri 2 Documentation](https://v2.tauri.app)

## License

MIT
