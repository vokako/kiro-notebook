use crate::acp::{self, AcpClient};
use crate::file_reader;
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use walkdir::WalkDir;

#[derive(Serialize, Deserialize, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub extension: String,
    pub children: Option<Vec<FileEntry>>,
}

#[tauri::command]
pub async fn select_workspace(
    path: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let canonical = std::fs::canonicalize(&path)
        .map_err(|e| format!("Invalid path: {}", e))?
        .display()
        .to_string();
    *state.workspace.lock().await = Some(canonical.clone());
    Ok(canonical)
}

#[tauri::command]
pub async fn list_files(
    state: tauri::State<'_, AppState>,
) -> Result<Vec<FileEntry>, String> {
    let workspace = state.workspace.lock().await;
    let workspace = workspace.as_ref().ok_or("No workspace selected")?;
    let extensions = ["pdf", "docx", "md", "txt", "html"];
    let mut entries: Vec<FileEntry> = Vec::new();

    for entry in WalkDir::new(workspace)
        .max_depth(3)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.to_str() == Some(workspace.as_str()) { continue; }
        if path.file_name().and_then(|n| n.to_str()).map(|n| n.starts_with('.')).unwrap_or(false) { continue; }

        if path.is_file() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if extensions.contains(&ext.to_lowercase().as_str()) {
                    entries.push(FileEntry {
                        name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                        path: path.display().to_string(),
                        is_dir: false,
                        extension: ext.to_lowercase(),
                        children: None,
                    });
                }
            }
        }
    }

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(entries)
}

#[tauri::command]
pub async fn read_file_content(path: String) -> Result<String, String> {
    file_reader::read_file(&path)
}

fn get_cwd(workspace: &tokio::sync::MutexGuard<'_, Option<String>>) -> Result<String, String> {
    workspace.as_ref().ok_or("No workspace selected".to_string()).cloned()
}

async fn spawn_and_init(_cwd: &str) -> Result<AcpClient, String> {
    let kiro_path = find_kiro_cli()?;
    let mut client = AcpClient::spawn(&kiro_path)?;
    client.initialize()?;
    Ok(client)
}

#[tauri::command]
pub async fn new_acp_session(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let cwd = get_cwd(&state.workspace.lock().await)?;
    let mut client = spawn_and_init(&cwd).await?;
    let session_id = client.new_session(&cwd)?;
    eprintln!("[CMD] New session: {}", session_id);

    let cancel_handle = client.cancel_handle();
    let mut sessions = state.sessions.lock().await;
    sessions.insert(session_id.clone(), Arc::new(Mutex::new(client)));
    drop(sessions);

    // Store cancel handle
    state.cancel_handles.lock().await.insert(session_id.clone(), (cancel_handle, session_id.clone()));
    Ok(session_id)
}

#[tauri::command]
pub async fn load_acp_session(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    if state.sessions.lock().await.contains_key(&session_id) {
        return Ok(session_id);
    }

    let cwd = get_cwd(&state.workspace.lock().await)?;
    let mut client = spawn_and_init(&cwd).await?;

    match client.load_session(&session_id, &cwd) {
        Ok(()) => {
            eprintln!("[CMD] Loaded session: {}", session_id);
            let cancel_handle = client.cancel_handle();
            state.sessions.lock().await.insert(session_id.clone(), Arc::new(Mutex::new(client)));
            state.cancel_handles.lock().await.insert(session_id.clone(), (cancel_handle, session_id.clone()));
            Ok(session_id)
        }
        Err(e) if e.contains("active in another process") => {
            drop(client);
            let lock_path = dirs::home_dir()
                .ok_or("Cannot find home dir")?
                .join(".kiro/sessions/cli")
                .join(format!("{}.lock", session_id));
            if lock_path.exists() {
                eprintln!("[CMD] Removing stale lock: {}", lock_path.display());
                let _ = std::fs::remove_file(&lock_path);
            }
            let mut client2 = spawn_and_init(&cwd).await?;
            client2.load_session(&session_id, &cwd)?;
            eprintln!("[CMD] Loaded session after lock removal: {}", session_id);
            let cancel_handle = client2.cancel_handle();
            state.sessions.lock().await.insert(session_id.clone(), Arc::new(Mutex::new(client2)));
            state.cancel_handles.lock().await.insert(session_id.clone(), (cancel_handle, session_id.clone()));
            Ok(session_id)
        }
        Err(e) => Err(e)
    }
}

#[tauri::command]
pub async fn send_prompt(
    session_id: String,
    message: String,
    context_files: Vec<String>,
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let client_arc = {
        let sessions = state.sessions.lock().await;
        sessions.get(&session_id).cloned().ok_or("Session not found")?
    };

    let mut client = client_arc.lock().await;

    let mut prompt = String::new();
    if !context_files.is_empty() {
        prompt.push_str("Here are the documents for context:\n\n");
        for f in &context_files {
            let name = std::path::Path::new(f).file_name().unwrap_or_default().to_string_lossy();
            match file_reader::read_file(f) {
                Ok(content) => {
                    let truncated = if content.len() > 50000 {
                        format!("{}...\n[truncated]", &content[..50000])
                    } else {
                        content
                    };
                    prompt.push_str(&format!("--- {} ---\n{}\n\n", name, truncated));
                }
                Err(e) => {
                    prompt.push_str(&format!("--- {} ---\n[Error reading: {}]\n\n", name, e));
                }
            }
        }
        prompt.push_str("---\n\nUser question: ");
    }
    prompt.push_str(&message);

    client.prompt_streaming(&prompt, &session_id, &app)
}

#[tauri::command]
pub async fn cancel_prompt(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let handles = state.cancel_handles.lock().await;
    let (stdin, sid) = handles.get(&session_id).ok_or("Session not found")?;
    acp::send_cancel(stdin, sid, 99999)
}

#[tauri::command]
pub async fn set_model(
    session_id: String,
    model_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let client_arc = {
        let sessions = state.sessions.lock().await;
        sessions.get(&session_id).cloned().ok_or("Session not found")?
    };
    let mut client = client_arc.lock().await;
    client.set_model(&model_id)
}

#[tauri::command]
pub async fn close_acp_session(
    session_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    state.cancel_handles.lock().await.remove(&session_id);
    if let Some(client_arc) = state.sessions.lock().await.remove(&session_id) {
        let mut client = client_arc.lock().await;
        client.kill();
        eprintln!("[CMD] Closed session: {}", session_id);
    }
    Ok(())
}

#[tauri::command]
pub async fn save_session_history(
    session_id: String,
    label: String,
    messages: String,
    context_files: Vec<String>,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let cwd = get_cwd(&state.workspace.lock().await)?;

    let dir = std::path::Path::new(&cwd).join(".kiro-notebook");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let path = dir.join(format!("{}.json", session_id));
    let data = serde_json::json!({
        "sessionId": session_id,
        "label": label,
        "messages": serde_json::from_str::<serde_json::Value>(&messages).unwrap_or_default(),
        "contextFiles": context_files,
        "updatedAt": chrono::Local::now().to_rfc3339(),
    });
    std::fs::write(&path, serde_json::to_string_pretty(&data).unwrap())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn load_session_history(
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    let cwd = get_cwd(&state.workspace.lock().await)?;

    let dir = std::path::Path::new(&cwd).join(".kiro-notebook");
    if !dir.exists() {
        return Ok("[]".to_string());
    }

    let mut sessions: Vec<serde_json::Value> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(entry.path()) {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                        sessions.push(val);
                    }
                }
            }
        }
    }

    sessions.sort_by(|a, b| {
        let ta = a.get("updatedAt").and_then(|v| v.as_str()).unwrap_or("");
        let tb = b.get("updatedAt").and_then(|v| v.as_str()).unwrap_or("");
        tb.cmp(ta)
    });

    Ok(serde_json::to_string(&sessions).unwrap())
}

fn find_kiro_cli() -> Result<String, String> {
    let candidates = [
        "/Applications/Kiro CLI.app/Contents/MacOS/kiro-cli",
        &format!("{}/.local/bin/kiro-cli", std::env::var("HOME").unwrap_or_default()),
        "/usr/local/bin/kiro-cli",
    ];

    for path in &candidates {
        if std::path::Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }

    if let Ok(output) = std::process::Command::new("which").arg("kiro-cli").output() {
        let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !path.is_empty() && std::path::Path::new(&path).exists() {
            return Ok(path);
        }
    }

    Err("kiro-cli not found. Please install Kiro CLI.".to_string())
}
