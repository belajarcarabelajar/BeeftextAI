use crate::snippet::Snippet;
use crate::store;
use crate::clipboard;
use crate::variable;
use crate::ollama::OllamaClient;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global toggle for substitution notifications
pub static NOTIFICATIONS_ENABLED: AtomicBool = AtomicBool::new(true);

/// Check if the typed buffer matches any snippet keyword and trigger substitution
pub async fn check_and_substitute(typed_buffer: &str, ollama: &OllamaClient, kb: &crate::keyboard::KeyboardState) -> bool {
    let snippets = match store::get_all_snippets() {
        Ok(s) => s,
        Err(_) => return false,
    };

    for snippet in &snippets {
        if !snippet.enabled {
            continue;
        }

        if snippet.matches_input(typed_buffer) {
            // Found a match — perform substitution
            kb.clear_buffer();
            perform_substitution(snippet, ollama).await;
            return true;
        }
    }

    false
}

/// Perform the text substitution
pub async fn perform_substitution(snippet: &Snippet, ollama: &OllamaClient) {
    // 1. Evaluate all variables in the snippet text
    let expanded = variable::evaluate_variables(&snippet.snippet, ollama).await;

    // 2. Erase the trigger keyword (backspace simulation)
    clipboard::erase_trigger(snippet.keyword.len());

    // 3. Inject the expanded text
    clipboard::inject_text(&expanded);

    // 4. Update last_used_at
    let mut updated = snippet.clone();
    updated.last_used_at = Some(chrono::Utc::now().to_rfc3339());
    let _ = store::update_snippet(&updated);

    // 5. Log
    let preview = if expanded.len() > 50 { format!("{}...", &expanded[..50]) } else { expanded.clone() };
    log::info!("Substituted '{}' → '{}'", snippet.keyword, preview);

    // 6. Show notification (if enabled)
    if NOTIFICATIONS_ENABLED.load(Ordering::Relaxed) {
        let title = format!("⚡ {}", if snippet.name.is_empty() { &snippet.keyword } else { &snippet.name });
        let body = if expanded.len() > 80 { format!("{}...", &expanded[..80]) } else { expanded };
        std::thread::spawn(move || {
            #[cfg(target_os = "windows")]
            {
                use std::process::Command;
                // Use PowerShell toast notification
                let ps_script = format!(
                    "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; \
                    $template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); \
                    $textNodes = $template.GetElementsByTagName('text'); \
                    $textNodes.Item(0).AppendChild($template.CreateTextNode('{}')) > $null; \
                    $textNodes.Item(1).AppendChild($template.CreateTextNode('{}')) > $null; \
                    $toast = [Windows.UI.Notifications.ToastNotification]::new($template); \
                    [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('BeefText AI').Show($toast)",
                    title.replace("'", "''"),
                    body.replace("'", "''")
                );
                use std::os::windows::process::CommandExt;
                
                // 0x08000000 is CREATE_NO_WINDOW, which prevents the brief console flash
                let _ = Command::new("powershell")
                    .args(["-WindowStyle", "Hidden", "-Command", &ps_script])
                    .creation_flags(0x08000000)
                    .spawn();
            }
        });
    }
}
