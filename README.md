<p align="center">
  <img src="icon.png" width="128" height="128" alt="KiroNotebook">
</p>

<h1 align="center">KiroNotebook</h1>

<p align="center">
  A local NotebookLM-style desktop app powered by <a href="https://kiro.dev/docs/cli/acp/">Kiro CLI's Agent Client Protocol (ACP)</a>
</p>

<p align="center">
  Built with <a href="https://v2.tauri.app">Tauri 2</a> · React · TypeScript · Rust
</p>

---

## What is this?

KiroNotebook lets you chat with AI about your local documents — PDFs, Word docs, Markdown, plain text, and HTML files — all without uploading anything to the cloud. It connects to [Kiro CLI](https://kiro.dev/cli/) running locally on your machine via the Agent Client Protocol.

Think of it as a local, privacy-friendly alternative to NotebookLM.

## Features

- **Three-panel layout** — File tree, document preview, and AI chat side by side
- **Document preview** — Native PDF/HTML rendering via iframe, Markdown with GFM support, DOCX text extraction
- **Context-aware chat** — Select files as context, AI receives their content with your questions
- **Per-session isolation** — Each chat session runs its own `kiro-cli acp` process, fully independent
- **Session management** — Multiple tabs, close sessions, persistent history to disk
- **Context tracking** — Sent files marked with ✓, new files queued until next message
- **Streaming responses** — Real-time token streaming from ACP
- **Cancel generation** — Stop AI mid-response with `session/cancel`
- **Model selection** — Switch between Claude Sonnet, Opus, Haiku, and Auto
- **Dark theme** — Easy on the eyes, with [Lucide](https://lucide.dev) icons throughout
- **Draggable divider** — Resize preview and chat panels to your preference

## Prerequisites

- [Kiro CLI](https://kiro.dev/downloads/) installed and authenticated
- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) toolchain

Verify Kiro CLI is available:

```bash
kiro-cli --version
```

## Getting Started

```bash
# Clone
git clone https://github.com/vokako/kiro-notebook.git
cd kiro-notebook

# Install dependencies
npm install

# Run in development mode
npm run tauri dev
```

## Build

```bash
# Build .app bundle (macOS)
npm run tauri build -- --bundles app

# The app will be at:
# src-tauri/target/release/bundle/macos/KiroNotebook.app
```

## How It Works

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  React UI   │────▶│   Tauri (Rust)   │────▶│  kiro-cli acp   │
│  (Frontend) │◀────│   (Commands)     │◀────│  (per session)  │
└─────────────┘     └──────────────────┘     └─────────────────┘
     events              invoke                  JSON-RPC 2.0
                                                 over stdio
```

1. **Open a workspace** — Select a folder containing your documents
2. **Select files** — Click files to preview, checkbox to add as AI context
3. **Chat** — Type a message, a new `kiro-cli acp` process spawns automatically
4. **Context is sent once** — Selected files are sent with your first message (or when newly added), marked with ✓ afterward
5. **Multiple sessions** — Each tab is an independent ACP process with its own conversation history
6. **History persists** — Sessions saved to `{workspace}/.kiro-notebook/` as JSON, restorable via 📋 button

## ACP Protocol

This app communicates with Kiro CLI using the [Agent Client Protocol](https://agentclientprotocol.com/), an open standard for agent-editor communication. Key methods used:

| Method | Purpose |
|--------|---------|
| `initialize` | Handshake and capability exchange |
| `session/new` | Create a new chat session |
| `session/load` | Restore a previous session (with full context) |
| `session/prompt` | Send a message, receive streaming response |
| `session/cancel` | Interrupt generation |
| `session/set_model` | Switch AI model |

See [`.agent/acp-reference.md`](.agent/acp-reference.md) for detailed protocol notes and test results.

## Project Structure

```
src/                    # React frontend
  App.tsx               # Main UI component
  App.css               # Dark theme styles
src-tauri/
  src/
    acp.rs              # ACP client: spawn, JSON-RPC, streaming
    commands.rs         # Tauri commands (bridge frontend ↔ ACP)
    file_reader.rs      # PDF/DOCX/MD/TXT content extraction
    lib.rs              # App state and plugin setup
  tauri.conf.json       # Tauri configuration
temp/                   # ACP protocol test scripts (Python)
.agent/                 # Architecture docs and ACP reference
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Desktop framework | [Tauri 2](https://v2.tauri.app) |
| Frontend | React 19 + TypeScript |
| Backend | Rust (Tokio async runtime) |
| AI backend | [Kiro CLI](https://kiro.dev/cli/) via ACP |
| Icons | [Lucide React](https://lucide.dev) |
| Markdown | react-markdown + remark-gfm |
| PDF extraction | pdf-extract (Rust) |
| DOCX extraction | docx-rs (Rust) |

## License

MIT
