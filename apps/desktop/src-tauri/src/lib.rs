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
mod token;

use ollama::{OllamaClient, ChatMessage};
use snippet::Snippet;
use group::Group;
use keyboard::KeyboardState;
use once_cell::sync::Lazy;
use std::sync::Arc;

static KEYBOARD: Lazy<Arc<KeyboardState>> = Lazy::new(|| Arc::new(KeyboardState::new()));

fn get_ollama() -> OllamaClient {
    let base_url = store::get_preference("ollama_url")
        .ok()
        .flatten()
        .unwrap_or_else(|| "http://localhost:11434".to_string());
    let text_model = store::get_preference("text_model")
        .ok()
        .flatten()
        .unwrap_or_else(|| "nemotron-3-super:cloud".to_string());
    let embed_model = store::get_preference("embed_model")
        .ok()
        .flatten()
        .unwrap_or_else(|| "nomic-embed-text".to_string());

    OllamaClient::new(base_url, text_model, embed_model)
}

const SYSTEM_PROMPT: &str = r#"You are an AI assistant for BeefText AI, a smart text expander.

### Snippet Creation
When the user wants to create/save a snippet, respond with:
{"keyword": "!abc", "snippet": "text", "name": "Name", "description": "Desc", "group": "GroupName", "content_type": "Text|Image|Both", "image_data": "base64_or_null"}
Always auto-generate a name, description, and assign a logical group.
For image snippets: set content_type to "Image" or "Both" and include the image_data field (the user already uploaded the image).
For text snippets: set content_type to "Text" and omit image_data.

### Keyword Rules
- Must start with ONE special symbol + exactly 3 letters (e.g., @eml, #sig, !add, %tyx, &brd)
- NO // prefix
- Be unique and memorable

### Snippet Context
User's existing snippets are provided below. Use them to avoid duplicates and answer questions.

### Template Variables
#{clipboard} #{date} #{time} #{dateTime:format} #{input:desc} #{combo:keyword} #{ai:prompt}"#;


// ─── Snippet Commands ─────────────────────────────────────────────────────────

#[tauri::command]
async fn get_snippets() -> Result<Vec<Snippet>, String> {
    store::get_all_snippets()
}

#[tauri::command]
async fn add_snippet(
    keyword: String,
    snippet_text: String,
    name: String,
    description: String,
    group_id: Option<String>,
    ai_generated: bool,
    image_data: Option<String>,
    content_type: Option<String>,
) -> Result<Snippet, String> {
    let mut s = Snippet::new(keyword, snippet_text, name, description, group_id);
    s.ai_generated = ai_generated;

    if let Some(ref b64) = image_data {
        s.image_data = Some(b64.clone());
    }
    if let Some(ref ct) = content_type {
        s.content_type = match ct.as_str() {
            "Image" => snippet::ContentType::Image,
            "Both" => snippet::ContentType::Both,
            _ => snippet::ContentType::Text,
        };
    }

    store::add_snippet(&s)?;

    let s_clone = s.clone();
    let client = get_ollama();
    tokio::spawn(async move {
        let text = format!(
            "name: {} | keyword: {} | description: {} | content: {}",
            s_clone.name, s_clone.keyword, s_clone.description, s_clone.snippet
        );
        match client.embed(vec![text]).await {
            Ok(embeddings) => {
                if let Some(emb) = embeddings.first() {
                    if let Err(e) = store::save_embedding(&s_clone.uuid, emb) {
                        eprintln!("[EMBED] Failed to save embedding for {}: {}", s_clone.uuid, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[EMBED] Failed to generate embedding for {}: {}", s_clone.uuid, e);
            }
        }
    });

    Ok(s)
}

#[tauri::command]
async fn update_snippet_cmd(s: Snippet, image_data: Option<String>) -> Result<(), String> {
    let mut updated = s;
    if let Some(ref b64) = image_data {
        updated.image_data = Some(b64.clone());
    }
    store::update_snippet(&updated)?;

    let s_clone = updated.clone();
    let client = get_ollama();
    tokio::spawn(async move {
        let text = format!(
            "name: {} | keyword: {} | description: {} | content: {}",
            s_clone.name, s_clone.keyword, s_clone.description, s_clone.snippet
        );
        match client.embed(vec![text]).await {
            Ok(embeddings) => {
                if let Some(emb) = embeddings.first() {
                    if let Err(e) = store::save_embedding(&s_clone.uuid, emb) {
                        eprintln!("[EMBED] Failed to save embedding for {}: {}", s_clone.uuid, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("[EMBED] Failed to generate embedding for {}: {}", s_clone.uuid, e);
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
async fn chat_with_ai(message: String, image_data: Option<String>) -> Result<String, String> {
    use token::{estimate_tokens, truncate_to_tokens, log_stats};

    // Truncate current message to 2000 tokens
    let message_truncated = truncate_to_tokens(&message, 2000);
    if message_truncated.len() < message.len() {
        eprintln!("[TOKEN] message truncated | original: {} chars | truncated: {} chars", message.len(), message_truncated.len());
    }

    store::save_chat_message("user", &message_truncated)?;

    let history = store::get_chat_history(50)?;
    let snippets = store::get_all_snippets().unwrap_or_default();
    let groups = store::get_all_groups().unwrap_or_default();

    // Build contexts with token budgets
    let group_context: String = groups.iter()
        .map(|g| format!("- {}", g.name))
        .collect::<Vec<_>>().join("\n");

    // Truncate snippets context: take top 50 snippets (by keyword/name match)
    let snippet_lines: Vec<String> = snippets.iter().take(50).map(|s| {
        format!("- Keyword: `{}` → \"{}\" ({})", s.keyword, s.snippet, s.name)
    }).collect();
    let snippet_context = snippet_lines.join("\n");

    // Build and truncate system prompt to 512 tokens
    let system_base = format!(
        "{}\n\nUser's existing groups:\n{}\n\nUser's existing snippets:\n{}",
        SYSTEM_PROMPT,
        if group_context.is_empty() { "None".to_string() } else { group_context },
        snippet_context
    );
    let system = truncate_to_tokens(&system_base, 512);

    log_stats("system", &system);
    log_stats("message", &message_truncated);

    // Budget: reserve 1024 tokens for response, 512 for system
    // nemotron-3-super:cloud uses 8K context = 8192 tokens max
    let max_context: usize = 8192;
    let system_tokens = estimate_tokens(&system);
    let response_budget = 1024;
    let history_budget = max_context.saturating_sub(system_tokens).saturating_sub(response_budget);

    // Build message list with token-aware history trimming
    let mut messages = vec![ChatMessage { role: "system".to_string(), content: system, images: None }];

    // Add history from newest to oldest, staying within budget
    let mut history_tokens_used = 0;
    for (role, content) in history.iter().rev() {
        let msg_tokens = estimate_tokens(content) + 10; // ~10 tokens overhead per message
        if history_tokens_used + msg_tokens > history_budget {
            break;
        }
        history_tokens_used += msg_tokens;
        messages.push(ChatMessage { role: role.clone(), content: content.clone(), images: None });
    }

    // Reverse to get chronological order (oldest first, newest last)
    // messages currently has: [system, ...recent_history_reversed]
    // We need to move system to front and keep history chronological
    let sys_msg = messages.remove(0);
    messages.reverse();
    messages.insert(0, sys_msg);

    messages.push(ChatMessage { role: "user".to_string(), content: message_truncated, images: None });

    let total_tokens: usize = messages.iter().map(|m| estimate_tokens(&m.content) + 10).sum();
    eprintln!("[TOKEN] total request | tokens: ~{} | messages: {}", total_tokens, messages.len());

    // Set num_ctx to limit context window on Ollama side
    let response = get_ollama().chat(messages, Some(max_context as i32)).await?;
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
    // Truncate query to avoid overflow and improve embedding quality
    let query_text = token::truncate_to_tokens(&query, 512);
    let query_embeddings = get_ollama().embed(vec![query_text]).await?;
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

/// Embedding configuration
const DEFAULT_EMBED_BATCH_SIZE: usize = 1;
const DEFAULT_EMBED_MAX_TOKENS: usize = 4096;

#[derive(Clone, Debug)]
struct EmbedConfig {
    /// Maximum tokens per text before truncation (default: 8192 for nomic-embed-text)
    max_tokens: usize,
    /// Number of snippets to batch in a single API call (default: 8)
    batch_size: usize,
    /// Partition number for distributed embedding (0 = all partitions, n = only partition n of total)
    partition: usize,
    /// Total partitions for distributed embedding
    total_partitions: usize,
}

impl Default for EmbedConfig {
    fn default() -> Self {
        Self {
            max_tokens: DEFAULT_EMBED_MAX_TOKENS,
            batch_size: DEFAULT_EMBED_BATCH_SIZE,
            partition: 0,
            total_partitions: 1,
        }
    }
}

fn make_embed_text(name: &str, keyword: &str, description: &str, snippet: &str, max_tokens: usize) -> String {
    // Combine all fields with labeled separators to help embedding model understand context
    let combined = format!(
        "name: {} | keyword: {} | description: {} | content: {}",
        name, keyword, description, snippet
    );
    token::truncate_to_tokens(&combined, max_tokens)
}

#[tauri::command]
async fn force_re_embed_all(resume: bool, app: tauri::AppHandle, batch_size: Option<usize>, max_tokens: Option<usize>, partition: Option<usize>, total_partitions: Option<usize>) -> Result<ReEmbedResult, String> {
    use tauri::Emitter;
    let all_snippets = store::get_all_snippets()?;
    let client = get_ollama();

    let config = EmbedConfig {
        max_tokens: max_tokens.unwrap_or(DEFAULT_EMBED_MAX_TOKENS),
        batch_size: batch_size.unwrap_or(DEFAULT_EMBED_BATCH_SIZE),
        partition: partition.unwrap_or(0),
        total_partitions: total_partitions.unwrap_or(1),
    };

    // Collect already-embedded UUIDs if resuming or filtering
    let embedded_uuids: std::collections::HashSet<String> = if resume || config.total_partitions > 1 {
        let embeddings = store::get_all_embeddings()?;
        embeddings.into_iter().map(|(id, _)| id).collect()
    } else {
        std::collections::HashSet::new()
    };

    // Filter snippets by partition and resume status
    let snippets: Vec<Snippet> = all_snippets.into_iter().enumerate().filter_map(|(i, s)| {
        // Partition filter
        if config.total_partitions > 1 && i % config.total_partitions != config.partition {
            return None;
        }
        // Resume filter: skip already embedded
        if resume && embedded_uuids.contains(&s.uuid) {
            return None;
        }
        Some(s)
    }).collect();

    let mut count = 0;
    let mut failures = Vec::new();

    let total = snippets.len();
    if total == 0 {
        return Ok(ReEmbedResult { successful: 0, failed: 0, failures: vec![] });
    }

    // Process in batches
    for batch_start in (0..total).step_by(config.batch_size) {
        let batch_end = (batch_start + config.batch_size).min(total);
        let batch = &snippets[batch_start..batch_end];

        // Prepare batch texts with truncation
        let texts: Vec<String> = batch.iter().map(|s| {
            make_embed_text(&s.name, &s.keyword, &s.description, &s.snippet, config.max_tokens)
        }).collect();

        let result = client.embed(texts).await;

        match result {
            Ok(embeddings) => {
                for (i, embedding) in embeddings.iter().enumerate() {
                    let snippet = &batch[i];
                    if store::save_embedding(&snippet.uuid, embedding).is_ok() {
                        count += 1;
                    } else {
                        failures.push(EmbedFailure {
                            uuid: snippet.uuid.clone(),
                            name: snippet.name.clone(),
                            reason: "Failed to save embedding to database".to_string(),
                        });
                    }
                }
            }
            Err(_e) => {
                // If batch fails, retry individually with more aggressive truncation
                for snippet in batch {
                    let text = make_embed_text(&snippet.name, &snippet.keyword, &snippet.description, &snippet.snippet, config.max_tokens / 2);
                    match client.embed(vec![text]).await {
                        Ok(embeddings) => {
                            if let Some(emb) = embeddings.first() {
                                if store::save_embedding(&snippet.uuid, emb).is_ok() {
                                    count += 1;
                                } else {
                                    failures.push(EmbedFailure {
                                        uuid: snippet.uuid.clone(),
                                        name: snippet.name.clone(),
                                        reason: "Failed to save embedding to database".to_string(),
                                    });
                                }
                            }
                        }
                        Err(e2) => {
                            failures.push(EmbedFailure {
                                uuid: snippet.uuid.clone(),
                                name: snippet.name.clone(),
                                reason: format!("Ollama API error: {} (tried halved context)", e2),
                            });
                        }
                    }
                }
            }
        }

        // Emit progress
        let current = batch_end;
        let _ = app.emit("embed_progress", EmbedProgress {
            current,
            total,
            percentage: (current as f64 / total as f64) * 100.0,
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
        .plugin(tauri_plugin_dialog::init())
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
