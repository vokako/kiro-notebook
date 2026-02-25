# Kiro Notebook Architecture

## Overview

Kiro Notebook is a local NotebookLM-style desktop app built with Tauri 2 + React + TypeScript.
It uses Kiro CLI's ACP (Agent Client Protocol) as the AI backend.

## Architecture

```
┌─────────────────────────────────────────────┐
│              Tauri Desktop App               │
│  ┌──────────┬──────────┬──────────────────┐  │
│  │ File List│ Preview  │    AI Chat       │  │
│  │ (left)   │ (center) │    (right)       │  │
│  └──────────┴──────────┴──────────────────┘  │
│                    │                          │
│              Tauri Backend (Rust)             │
│                    │                          │
│         ┌─────────┴─────────┐                │
│         │                   │                │
│    ACP (stdio)         MCP Server            │
│         │              (file reading)        │
│    Kiro CLI ◄──────── file tools             │
│   (AI reasoning)                             │
└─────────────────────────────────────────────┘
```

## Key Components

### Rust Backend (`src-tauri/src/`)
- `lib.rs` - App entry point, state management, plugin registration
- `acp.rs` - ACP client: spawns `kiro-cli acp`, communicates via JSON-RPC over stdio
- `commands.rs` - Tauri commands exposed to frontend (workspace, files, ACP)
- `file_reader.rs` - File content extraction (PDF, DOCX, MD, TXT, HTML)
- `mcp_server.rs` - MCP stdio server providing file tools to Kiro CLI

### React Frontend (`src/`)
- `App.tsx` - Main app with three-panel layout
- `App.css` - Dark theme styling

### MCP Server Mode
The app binary itself doubles as an MCP server when invoked with `--mcp-server`.
This is passed to Kiro CLI during ACP session creation so Kiro can read user documents.

### ACP Flow
1. User clicks "Connect AI" → `start_acp` command
2. Rust spawns `kiro-cli acp` subprocess
3. Sends `initialize` JSON-RPC request
4. Creates session with `session/new`, attaching MCP server
5. User sends message → `session/prompt` → collects `session/update` notifications
6. Agent message chunks are concatenated and returned to frontend

### MCP Tools
- `read_file(path)` - Read file content (auto-handles PDF/DOCX/MD/TXT)
- `list_files(directory)` - List supported files in directory
- `search_content(query, files)` - Search text across files
