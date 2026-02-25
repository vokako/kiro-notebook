# ACP (Agent Client Protocol) Reference

Source: https://kiro.dev/docs/cli/acp/

## Overview

Kiro CLI implements ACP, communicating over stdin/stdout using JSON-RPC 2.0.

```bash
kiro-cli acp
```

## Core Methods

| Method | Description |
|--------|-------------|
| `initialize` | Initialize connection and exchange capabilities |
| `session/new` | Create a new chat session |
| `session/load` | Load an existing session by ID |
| `session/prompt` | Send a prompt to the agent |
| `session/cancel` | Cancel the current operation |
| `session/set_mode` | Switch agent mode |
| `session/set_model` | Change the model for the session |

## Agent Capabilities (returned by initialize)

- `loadSession: true` — supports loading existing sessions
- `promptCapabilities.image: true` — supports image content

## Session Updates (notifications, no id field)

| Update Type | Description |
|-------------|-------------|
| `AgentMessageChunk` / `agent_message_chunk` | Streaming text from agent |
| `ToolCall` / `tool_call` | Tool invocation |
| `ToolCallUpdate` / `tool_call_update` | Progress updates for tools |
| `TurnEnd` / `turn_end` | Agent turn completed |

## Kiro Extensions (optional, prefixed `_kiro.dev/`)

- `_kiro.dev/commands/execute` — Execute slash command
- `_kiro.dev/commands/options` — Autocomplete suggestions
- `_kiro.dev/commands/available` — Notification: available commands after session creation
- `_kiro.dev/mcp/oauth_request` — OAuth URL for MCP auth
- `_kiro.dev/mcp/server_initialized` — MCP server ready
- `_kiro.dev/compaction/status` — Context compaction progress
- `_kiro.dev/clear/status` — Session history clear status
- `_session/terminate` — Terminate subagent session

## Protocol Examples

### Initialize

```json
// Request
{"jsonrpc":"2.0","id":0,"method":"initialize","params":{
  "protocolVersion":1,
  "clientCapabilities":{},
  "clientInfo":{"name":"my-app","version":"0.1.0"}
}}

// Response
{"jsonrpc":"2.0","id":0,"result":{
  "protocolVersion":1,
  "agentCapabilities":{"loadSession":true,"promptCapabilities":{"image":true}},
  "agentInfo":{"name":"Kiro Agent","version":"1.26.2"}
}}
```

### session/new

```json
// Request
{"jsonrpc":"2.0","id":1,"method":"session/new","params":{
  "cwd":"/path/to/project",
  "mcpServers":[]
}}

// Response
{"jsonrpc":"2.0","id":1,"result":{"sessionId":"uuid-here"}}
```

### session/prompt

**IMPORTANT**: Parameter name is `prompt`, NOT `content`.

```json
// Request
{"jsonrpc":"2.0","id":2,"method":"session/prompt","params":{
  "sessionId":"uuid-here",
  "prompt":[{"type":"text","text":"hello"}]
}}

// Streaming notifications arrive before response:
{"jsonrpc":"2.0","method":"session/update","params":{
  "update":{"sessionUpdate":"agent_message_chunk","content":{"text":"chunk..."}}
}}

// Final response
{"jsonrpc":"2.0","id":2,"result":{"stopReason":"end_turn"}}
```

### session/load

Loads a previously created session in a new process. Session must NOT be locked by another process.

```json
// Request
{"jsonrpc":"2.0","id":1,"method":"session/load","params":{
  "sessionId":"uuid-here",
  "cwd":"/path/to/project",
  "mcpServers":[]
}}

// Response replays history via session/update notifications, then:
{"jsonrpc":"2.0","id":1,"result":{"sessionId":"uuid-here","modes":{...}}}
```

### session/set_model

**IMPORTANT**: Parameter name is `modelId`, NOT `model`.

```json
{"jsonrpc":"2.0","id":3,"method":"session/set_model","params":{
  "sessionId":"uuid-here",
  "modelId":"claude-sonnet-4"
}}
```

Available models (as of 2026-02):
- `auto` — 1.00x credits, task-optimal selection
- `claude-opus-4.6` — 2.20x credits
- `claude-sonnet-4.6` — 1.30x credits
- `claude-opus-4.5` — 2.20x credits
- `claude-sonnet-4.5` — 1.30x credits
- `claude-sonnet-4` — 1.30x credits
- `claude-haiku-4.5` — 0.40x credits

## Session Storage

Sessions persisted at: `~/.kiro/sessions/cli/`

Each session has:
- `<session-id>.json` — metadata and state
- `<session-id>.jsonl` — event log (conversation history)
- `<session-id>.lock` — process lock (contains `{"pid":N,"started_at":"..."}`)

**Lock behavior**: A session cannot be loaded if another process holds the lock. The lock is released when the process exits. `kiro-cli` spawns a child process `kiro-cli-chat` — both must be killed to release the lock.

## Logging

- macOS: `$TMPDIR/kiro-log/kiro-chat.log`
- Linux: `$XDG_RUNTIME_DIR/kiro-log/kiro-chat.log`
- Debug: `KIRO_LOG_LEVEL=debug kiro-cli acp`

## Test Results (2026-02-25)

| Test | Result | Notes |
|------|--------|-------|
| New session + prompt | ✅ | `prompt` param, not `content` |
| Load session (cross-process) | ✅ | Works after old process fully exits and lock released |
| Set model | ✅ | `modelId` param, not `model` |
| Streaming | ✅ | Chunks arrive via `session/update` notifications |
| Load locked session | ❌ Expected | "Session is active in another process (PID X)" |
