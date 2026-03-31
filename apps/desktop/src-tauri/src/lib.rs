mod snippet;
mod group;
mod ollama;
mod store;
mod migration;
mod keyboard;
mod engine;
mod clipboard;
mod variable;
mod backup;
mod trigger;

use ollama::{OllamaClient, ChatMessage};
use snippet::Snippet;
use group::Group;
use keyboard::KeyboardState;
use once_cell::sync::Lazy;
use std::sync::Arc;

static KEYBOARD: Lazy<Arc<KeyboardState>> = Lazy::new(|| Arc::new(KeyboardState::new()));

fn get_ollama() -> OllamaClient {
    OllamaClient::new(
        "http://localhost:11434".to_string(),
        "nemotron-3-super:cloud".to_string(),
        "nomic-embed-text".to_string(),
    )
}

const SYSTEM_PROMPT: &str = r#"You are an AI assistant for BeefText AI, a smart text expander.
Your goal is to help users manage their shortcuts (snippets) and answer questions about them.

### Capabilities:
1. **Create/Update Snippets**: When a user wants to save or create a shortcut, respond with a JSON object followed by a brief explanation.
   Format: {"keyword": "!abc", "snippet": "expanded text", "name": "Name", "description": "Description", "group": "Group"}
   You MUST auto-generate a suitable `name`, `description`, and assign it to a logical `group`.
   If one of the "User's existing groups" matches the context perfectly, use that exact group name. If no existing group fits, generate a concise new group name.
2. **Query Snippets**: You can see the user's existing snippets below. If they ask "What is the shortcut for X?" or "List my email shortcuts", answer based on the provided context.
3. **General Assistance**: Answer questions about how to use BeefText AI, its variables, or general productivity tips.

### Guidelines:
- Suggest short, memorable, and highly unique keywords. The keyword MUST start with EXACTLY ONE special symbol followed by exactly 3 letters (e.g., `@eml`, `#sig`, `!add`, `%tyx`, `&brd`). DO NOT use the `//` prefix.
- Auto-generate meaningful and descriptive `name` and `description` for every snippet to provide useful context.
- Use the provided context of "User's existing snippets" to avoid duplicates and answer questions accurately.
- Be concise and helpful. Respond in the same language the user uses.

### Template Variables:
- #{clipboard} — Current clipboard
- #{date}, #{time}, #{dateTime:format} — Date/Time
- #{input:description} — User input on trigger
- #{combo:keyword} — Recursive snippet
- #{ai:prompt} — Dynamic AI generation on trigger"#;


// ─── Snippet Commands ─────────────────────────────────────────────────────────

#[tauri::command]
async fn get_snippets() -> Result<Vec<Snippet>, String> {
    store::get_all_snippets()
}

#[tauri::command]
async fn add_snippet(keyword: String, snippet_text: String, name: String, description: String, group_id: Option<String>, ai_generated: bool) -> Result<Snippet, String> {
    let mut s = Snippet::new(keyword, snippet_text, name, description, group_id);
    s.ai_generated = ai_generated;
    store::add_snippet(&s)?;
    
    let s_clone = s.clone();
    let client = get_ollama();
    tokio::spawn(async move {
        let text = format!("{} {} {} {}", s_clone.name, s_clone.keyword, s_clone.description, s_clone.snippet);
        if let Ok(embeddings) = client.embed(vec![text]).await {
            if let Some(emb) = embeddings.first() {
                let _ = store::save_embedding(&s_clone.uuid, emb);
            }
        }
    });
    
    Ok(s)
}

#[tauri::command]
async fn update_snippet_cmd(s: Snippet) -> Result<(), String> {
    store::update_snippet(&s)?;
    
    let s_clone = s.clone();
    let client = get_ollama();
    tokio::spawn(async move {
        let text = format!("{} {} {} {}", s_clone.name, s_clone.keyword, s_clone.description, s_clone.snippet);
        if let Ok(embeddings) = client.embed(vec![text]).await {
            if let Some(emb) = embeddings.first() {
                let _ = store::save_embedding(&s_clone.uuid, emb);
            }
        }
    });
    Ok(())
}

#[tauri::command]
async fn delete_snippet_cmd(uuid: String) -> Result<(), String> {
    store::delete_snippet(&uuid)
}

#[tauri::command]
async fn toggle_snippet_enabled(uuid: String, enabled: bool) -> Result<(), String> {
    store::toggle_snippet_enabled(&uuid, enabled)
}

// ─── Group Commands ───────────────────────────────────────────────────────────

#[tauri::command]
async fn get_groups() -> Result<Vec<Group>, String> {
    store::get_all_groups()
}

#[tauri::command]
async fn add_group_cmd(name: String, description: String) -> Result<Group, String> {
    let g = Group::new(name, description);
    store::add_group(&g)?;
    Ok(g)
}

#[tauri::command]
async fn update_group_cmd(g: Group) -> Result<(), String> {
    store::update_group(&g)
}

#[tauri::command]
async fn delete_group_cmd(uuid: String, #[allow(non_snake_case)] deleteSnippets: bool) -> Result<(), String> {
    store::delete_group(&uuid, deleteSnippets)
}

#[tauri::command]
async fn delete_snippets_in_group_cmd(group_uuid: String) -> Result<usize, String> {
    store::delete_snippets_in_group(&group_uuid)
}

// ─── Ollama Commands ──────────────────────────────────────────────────────────

#[tauri::command]
async fn ollama_status() -> Result<bool, String> {
    Ok(get_ollama().is_available().await)
}

#[tauri::command]
async fn ollama_models() -> Result<Vec<ollama::OllamaModel>, String> {
    get_ollama().list_models().await
}

#[tauri::command]
async fn chat_with_ai(message: String) -> Result<String, String> {
    store::save_chat_message("user", &message)?;
    
    let history = store::get_chat_history(20)?;
    let snippets = store::get_all_snippets().unwrap_or_default();
    let snippet_context: String = snippets.iter().take(100).map(|s| {
        format!("- Keyword: `{}` → \"{}\" ({})", s.keyword, s.snippet, s.name)
    }).collect::<Vec<_>>().join("\n");
    
    let groups = store::get_all_groups().unwrap_or_default();
    let group_context: String = groups.iter().map(|g| {
        format!("- {}", g.name)
    }).collect::<Vec<_>>().join("\n");
    
    let system = format!(
        "{}\n\nUser's existing groups:\n{}\n\nUser's existing snippets:\n{}", 
        SYSTEM_PROMPT, 
        if group_context.is_empty() { "None".to_string() } else { group_context },
        snippet_context
    );
    
    let mut messages = vec![ChatMessage { role: "system".to_string(), content: system }];
    for (role, content) in &history {
        messages.push(ChatMessage { role: role.clone(), content: content.clone() });
    }
    messages.push(ChatMessage { role: "user".to_string(), content: message });
    
    let response = get_ollama().chat(messages).await?;
    store::save_chat_message("assistant", &response.content)?;
    Ok(response.content)
}

#[tauri::command]
async fn clear_chat() -> Result<(), String> {
    store::clear_chat_history()
}

#[tauri::command]
async fn get_chat_history_cmd() -> Result<Vec<(String, String)>, String> {
    store::get_chat_history(100)
}

// ─── Semantic Search ──────────────────────────────────────────────────────────

#[tauri::command]
async fn semantic_search(query: String, limit: usize) -> Result<Vec<(String, f32)>, String> {
    let query_embeddings = get_ollama().embed(vec![query]).await?;
    let query_emb = query_embeddings.first().ok_or("No embedding returned")?;
    let stored = store::get_all_embeddings()?;
    
    let mut scores: Vec<(String, f32)> = stored.iter().map(|(uuid, emb)| {
        let sim = cosine_similarity(query_emb, emb);
        (uuid.clone(), sim)
    }).collect();
    
    scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scores.truncate(limit);
    Ok(scores)
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let mag_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let mag_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if mag_a == 0.0 || mag_b == 0.0 { return 0.0; }
    dot / (mag_a * mag_b)
}

// ─── Import / Export ──────────────────────────────────────────────────────────

#[tauri::command]
async fn import_beeftext(json_content: String) -> Result<migration::ImportResult, String> {
    migration::import_beeftext_json(&json_content)
}

#[tauri::command]
async fn export_json() -> Result<String, String> {
    migration::export_all_as_json()
}

#[tauri::command]
async fn export_csv() -> Result<String, String> {
    migration::export_as_csv()
}

#[tauri::command]
async fn generate_cheat_sheet() -> Result<String, String> {
    migration::generate_cheat_sheet()
}

// ─── Keyboard Hook Control ───────────────────────────────────────────────────

#[tauri::command]
async fn start_keyboard_hook() -> Result<String, String> {
    // Ensure worker is running
    trigger::ensure_worker_running();

    let kb = Arc::clone(&KEYBOARD);
    trigger::set_keyboard_state(Arc::clone(&KEYBOARD));

    kb.start_listening(move |buffer| {
        trigger::enqueue_trigger(buffer);
    });

    Ok("Keyboard hook started".to_string())
}

#[tauri::command]
async fn stop_keyboard_hook() -> Result<String, String> {
    KEYBOARD.set_enabled(false);
    Ok("Keyboard hook disabled".to_string())
}

#[tauri::command]
async fn toggle_keyboard_hook(enabled: bool) -> Result<bool, String> {
    KEYBOARD.set_enabled(enabled);
    Ok(enabled)
}

#[tauri::command]
async fn is_keyboard_hook_active() -> Result<bool, String> {
    Ok(KEYBOARD.is_enabled())
}

#[tauri::command]
async fn toggle_notifications(enabled: bool) -> Result<bool, String> {
    engine::NOTIFICATIONS_ENABLED.store(enabled, std::sync::atomic::Ordering::Relaxed);
    Ok(enabled)
}

#[tauri::command]
async fn is_notifications_enabled() -> Result<bool, String> {
    Ok(engine::NOTIFICATIONS_ENABLED.load(std::sync::atomic::Ordering::Relaxed))
}

// ─── Stats ────────────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_snippet_count_by_group() -> Result<Vec<(Option<String>, i64)>, String> {
    store::get_snippet_count_by_group()
}

#[tauri::command]
async fn get_snippet_stats() -> Result<(i64, i64, i64, i64), String> {
    store::get_snippet_stats()
}

#[derive(Clone, serde::Serialize)]
struct EmbedProgress {
    current: usize,
    total: usize,
    percentage: f64,
}

#[derive(serde::Serialize)]
struct EmbedFailure {
    uuid: String,
    name: String,
    reason: String,
}

#[derive(serde::Serialize)]
struct ReEmbedResult {
    successful: usize,
    failed: usize,
    failures: Vec<EmbedFailure>,
}

#[tauri::command]
async fn force_re_embed_all(resume: bool, app: tauri::AppHandle) -> Result<ReEmbedResult, String> {
    use tauri::Emitter;
    let mut snippets = store::get_all_snippets()?;
    let client = get_ollama();
    let mut count = 0;
    let mut failures = Vec::new();

    if resume {
        let embeddings = store::get_all_embeddings()?;
        let embedded_uuids: std::collections::HashSet<String> = embeddings.into_iter().map(|(id, _)| id).collect();
        snippets.retain(|s| !embedded_uuids.contains(&s.uuid));
    }

    let total = snippets.len();
    if total == 0 {
        return Ok(ReEmbedResult { successful: 0, failed: 0, failures: vec![] });
    }

    for (i, s) in snippets.iter().enumerate() {
        let text = format!("{} {} {} {}", s.name, s.keyword, s.description, s.snippet);
        let result = client.embed(vec![text]).await;

        match result {
            Ok(embeddings) => {
                if let Some(emb) = embeddings.first() {
                    if store::save_embedding(&s.uuid, emb).is_ok() {
                        count += 1;
                    } else {
                        failures.push(EmbedFailure {
                            uuid: s.uuid.clone(),
                            name: s.name.clone(),
                            reason: "Failed to save embedding to database".to_string(),
                        });
                    }
                } else {
                    failures.push(EmbedFailure {
                        uuid: s.uuid.clone(),
                        name: s.name.clone(),
                        reason: "Ollama returned empty embeddings array".to_string(),
                    });
                }
            }
            Err(e) => {
                failures.push(EmbedFailure {
                    uuid: s.uuid.clone(),
                    name: s.name.clone(),
                    reason: format!("Ollama API error: {}", e),
                });
            }
        }

        // Emit progress
        let _ = app.emit("embed_progress", EmbedProgress {
            current: i + 1,
            total,
            percentage: ((i + 1) as f64 / total as f64) * 100.0,
        });
    }

    let failed = failures.len();
    for f in &failures {
        eprintln!("Embedding failed for snippet '{}' ({}): {}", f.name, f.uuid, f.reason);
    }

    Ok(ReEmbedResult { successful: count, failed, failures })
}

#[tauri::command]
async fn clear_all_data() -> Result<(), String> {
    store::clear_all_data()
}

// ─── Backup / Restore ─────────────────────────────────────────────────────────

#[tauri::command]
async fn create_backup() -> Result<backup::BackupInfo, String> {
    backup::create_backup()
}

#[tauri::command]
async fn list_backups() -> Result<Vec<backup::BackupInfo>, String> {
    backup::list_backups()
}

#[tauri::command]
async fn restore_backup_cmd(filename: String) -> Result<(usize, usize), String> {
    backup::restore_backup(&filename)
}

#[tauri::command]
async fn delete_backup_cmd(filename: String) -> Result<(), String> {
    backup::delete_backup(&filename)
}

#[tauri::command]
async fn restore_from_json_cmd(json_content: String) -> Result<(usize, usize), String> {
    backup::restore_from_json(&json_content)
}

// ─── Preferences ──────────────────────────────────────────────────────────────

#[tauri::command]
async fn get_preference(key: String) -> Result<Option<String>, String> {
    store::get_preference(&key)
}

#[tauri::command]
async fn set_preference(key: String, value: String) -> Result<(), String> {
    store::set_preference(&key, &value)
}

// ─── App Setup ────────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::init();

    let db_dir = dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("BeeftextAI");
    std::fs::create_dir_all(&db_dir).ok();
    let db_path = db_dir.join("beeftextai.db");
    
    if let Err(e) = store::init_db(db_path.to_str().unwrap_or("beeftextai.db")) {
        eprintln!("Failed to initialize database: {}", e);
    }

    // Auto-start keyboard hook
    trigger::ensure_worker_running();
    trigger::set_keyboard_state(Arc::clone(&KEYBOARD));

    let kb = Arc::clone(&KEYBOARD);
    kb.start_listening(move |buffer| {
        trigger::enqueue_trigger(buffer);
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {

            use tauri::menu::{MenuBuilder, MenuItemBuilder};
            use tauri::tray::TrayIconBuilder;
            use tauri::Manager;

            // Build tray menu
            let show_hide = MenuItemBuilder::with_id("show_hide", "👁️ Show / Hide Window").build(app)?;
            let toggle_expander = tauri::menu::CheckMenuItemBuilder::with_id("toggle_expander", "⌨️ Text Expander Active")
                .checked(true)
                .build(app)?;
            let separator = tauri::menu::PredefinedMenuItem::separator(app)?;
            let quit = MenuItemBuilder::with_id("quit", "🚪 Quit BeefText AI").build(app)?;

            let menu = MenuBuilder::new(app)
                .items(&[&show_hide, &toggle_expander, &separator, &quit])
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("BeefText AI — Smart Text Expander")
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "show_hide" => {
                            if let Some(window) = app.get_webview_window("main") {
                                if window.is_visible().unwrap_or(false) {
                                    let _ = window.hide();
                                } else {
                                    let _ = window.show();
                                    let _ = window.set_focus();
                                }
                            }
                        }
                        "toggle_expander" => {
                            let current = KEYBOARD.is_enabled();
                            let new_state = !current;
                            KEYBOARD.set_enabled(new_state);
                            
                            // Visual feedback in the Tray Menu
                            if let Some(item) = app.menu().unwrap().get("toggle_expander") {
                                if let Some(check_item) = item.as_check_menuitem() {
                                    let new_text = if new_state { "⌨️ Text Expander Active" } else { "⏸ Text Expander Paused" };
                                    let _ = check_item.set_text(new_text);
                                }
                            }
                        }
                        "quit" => {
                            std::process::exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    use tauri::tray::TrayIconEvent;
                    if let TrayIconEvent::Click { button: tauri::tray::MouseButton::Left, .. } = event {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            // Minimize to tray on close (don't actually quit)
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_snippets,
            add_snippet,
            update_snippet_cmd,
            delete_snippet_cmd,
            toggle_snippet_enabled,
            get_groups,
            add_group_cmd,
            update_group_cmd,
            delete_group_cmd,
            delete_snippets_in_group_cmd,
            ollama_status,
            ollama_models,
            chat_with_ai,
            clear_chat,
            get_chat_history_cmd,
            semantic_search,
            import_beeftext,
            export_json,
            export_csv,
            generate_cheat_sheet,
            start_keyboard_hook,
            stop_keyboard_hook,
            toggle_keyboard_hook,
            is_keyboard_hook_active,
            toggle_notifications,
            is_notifications_enabled,
            get_snippet_count_by_group,
            get_snippet_stats,
            force_re_embed_all,
            create_backup,
            list_backups,
            restore_backup_cmd,
            delete_backup_cmd,
            restore_from_json_cmd,
            get_preference,
            set_preference,
            clear_all_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
