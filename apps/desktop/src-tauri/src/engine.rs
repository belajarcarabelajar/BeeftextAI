use crate::snippet::Snippet;
use crate::store;
use crate::clipboard;
use crate::variable;
use crate::ollama::OllamaClient;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global toggle for substitution notifications
pub static NOTIFICATIONS_ENABLED: AtomicBool = AtomicBool::new(true);

/// Perform the text substitution
pub async fn perform_substitution(snippet: &Snippet, ollama: &OllamaClient) {
    use crate::snippet::ContentType;

    // Evaluate variables only if there's text content
    let expanded = if !snippet.snippet.is_empty() {
        variable::evaluate_variables(&snippet.snippet, ollama).await
    } else {
        String::new()
    };

    // Erase the trigger keyword (backspace simulation)
    clipboard::erase_trigger(snippet.keyword.len());

    // Inject content based on type
    match snippet.content_type {
        ContentType::Text => {
            clipboard::inject_text(&expanded);
        }
        ContentType::Image => {
            if let Some(ref b64) = snippet.image_data {
                clipboard::inject_image(b64);
            }
        }
        ContentType::Both => {
            // Inject text first, then image after delay
            if !expanded.is_empty() {
                clipboard::inject_text(&expanded);
            }
            if let Some(ref b64) = snippet.image_data {
                std::thread::sleep(std::time::Duration::from_millis(150));
                clipboard::inject_image(b64);
            }
        }
    }

    // Update last_used_at
    let mut updated = snippet.clone();
    updated.last_used_at = Some(chrono::Utc::now().to_rfc3339());
    let _ = store::update_snippet(&updated);

    // Log
    let preview = if expanded.len() > 50 { format!("{}...", &expanded[..50]) } else { expanded.clone() };
    let content_desc = match snippet.content_type {
        ContentType::Text => format!("'{}'", preview),
        ContentType::Image => "[image]".to_string(),
        ContentType::Both => format!("'{}' + [image]", preview),
    };
    log::info!("Substituted '{}' → {}", snippet.keyword, content_desc);

    // Show notification (if enabled)
    if NOTIFICATIONS_ENABLED.load(Ordering::Relaxed) {
        let title = format!("⚡ {}", if snippet.name.is_empty() { &snippet.keyword } else { &snippet.name });
        let body = match snippet.content_type {
            ContentType::Text => if expanded.len() > 80 { format!("{}...", &expanded[..80]) } else { expanded },
            ContentType::Image => "[image]".to_string(),
            ContentType::Both => "[text + image]".to_string(),
        };
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
