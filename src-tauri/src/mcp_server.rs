use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

use crate::file_reader;

pub fn run_mcp_server() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let request: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = request.get("id").cloned();

        let result = match method {
            "initialize" => json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "kiro-notebook-files", "version": "0.1.0" }
            }),
            "tools/list" => json!({
                "tools": [
                    {
                        "name": "read_file",
                        "description": "Read the text content of a file. Supports PDF, DOCX, MD, TXT, HTML files.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string", "description": "Absolute path to the file" }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "list_files",
                        "description": "List files in a directory. Returns file names and paths.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "directory": { "type": "string", "description": "Absolute path to directory" }
                            },
                            "required": ["directory"]
                        }
                    },
                    {
                        "name": "search_content",
                        "description": "Search for text across multiple files. Returns matching lines with file paths.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "query": { "type": "string", "description": "Text to search for" },
                                "files": {
                                    "type": "array",
                                    "items": { "type": "string" },
                                    "description": "List of absolute file paths to search in"
                                }
                            },
                            "required": ["query", "files"]
                        }
                    }
                ]
            }),
            "tools/call" => handle_tool_call(&request),
            "notifications/initialized" => {
                continue;
            }
            _ => json!({"error": format!("Unknown method: {}", method)}),
        };

        if let Some(id) = id {
            let response = json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            });
            let msg = format!("{}\n", serde_json::to_string(&response).unwrap());
            let _ = stdout.write_all(msg.as_bytes());
            let _ = stdout.flush();
        }
    }
}

fn handle_tool_call(request: &Value) -> Value {
    let params = request.get("params").unwrap_or(&Value::Null);
    let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let arguments = params.get("arguments").unwrap_or(&Value::Null);

    match tool_name {
        "read_file" => {
            let path = arguments.get("path").and_then(|v| v.as_str()).unwrap_or("");
            match file_reader::read_file(path) {
                Ok(content) => json!({
                    "content": [{ "type": "text", "text": content }]
                }),
                Err(e) => json!({
                    "content": [{ "type": "text", "text": format!("Error: {}", e) }],
                    "isError": true
                }),
            }
        }
        "list_files" => {
            let dir = arguments.get("directory").and_then(|v| v.as_str()).unwrap_or("");
            let extensions = ["pdf", "docx", "md", "txt", "html"];
            let mut files = Vec::new();

            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if extensions.contains(&ext.to_lowercase().as_str()) {
                                files.push(path.display().to_string());
                            }
                        }
                    }
                }
            }

            json!({
                "content": [{ "type": "text", "text": files.join("\n") }]
            })
        }
        "search_content" => {
            let query = arguments
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let files = arguments
                .get("files")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let query_lower = query.to_lowercase();
            let mut results = Vec::new();

            for file_path in files {
                if let Ok(content) = file_reader::read_file(file_path) {
                    for (i, line) in content.lines().enumerate() {
                        if line.to_lowercase().contains(&query_lower) {
                            results.push(format!("{}:{}: {}", file_path, i + 1, line));
                        }
                    }
                }
            }

            json!({
                "content": [{ "type": "text", "text": results.join("\n") }]
            })
        }
        _ => json!({
            "content": [{ "type": "text", "text": format!("Unknown tool: {}", tool_name) }],
            "isError": true
        }),
    }
}
