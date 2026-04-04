// engine.rs — Super Ultra Plan: P12 (Fragment-Based Rendering)
//
// P12: Split the substitution pipeline into fragments so that #{key:}, #{delay:},
//      and #{shortcut:} fire at the correct position within the text, not before
//      or after the entire paste. This mirrors the original Beeftext's
//      splitStringIntoSnippetFragments() + renderSnippetFragmentList().

use crate::snippet::Snippet;
use crate::store;
use crate::clipboard;
use crate::variable::{self, SnippetFragment};
use crate::ollama::OllamaClient;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// Global toggle for substitution notifications
pub static NOTIFICATIONS_ENABLED: AtomicBool = AtomicBool::new(true);

/// Extension trait for character-aware string operations
trait StrExt {
    fn char_count(&self) -> usize;
    fn truncate_chars(&self, max_chars: usize) -> String;
}

impl StrExt for String {
    fn char_count(&self) -> usize {
        self.chars().count()
    }

    fn truncate_chars(&self, max_chars: usize) -> String {
        self.chars().take(max_chars).collect()
    }
}

/// P12: Render a list of snippet fragments sequentially.
/// Text fragments are injected via clipboard paste.
/// Key/Delay/Shortcut fragments are executed in-place.
/// This mirrors the original Beeftext's renderSnippetFragmentList().
fn render_fragments(fragments: &[SnippetFragment], cursor_offset: Option<i32>) {
    // Accumulate text fragments into one paste for efficiency.
    // When we hit a non-text fragment, flush the accumulated text first.
    let mut text_accumulator = String::new();

    for fragment in fragments {
        match fragment {
            SnippetFragment::Text(text) => {
                text_accumulator.push_str(text);
            }
            SnippetFragment::KeyPress(key, count) => {
                // Flush accumulated text before key press
                if !text_accumulator.is_empty() {
                    clipboard::inject_text(&text_accumulator);
                    text_accumulator.clear();
                    // Rapid-fire: reduced from 30ms to 10ms for faster fragment chaining
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                // Simulate key press(es)
                for _ in 0..*count {
                    clipboard::simulate_key_press(*key);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            SnippetFragment::Delay(ms) => {
                // Flush accumulated text before delay
                if !text_accumulator.is_empty() {
                    clipboard::inject_text(&text_accumulator);
                    text_accumulator.clear();
                    // Rapid-fire: reduced from 30ms to 10ms for faster fragment chaining
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                std::thread::sleep(std::time::Duration::from_millis(*ms));
            }
            SnippetFragment::Shortcut(modifiers, key) => {
                // Flush accumulated text before shortcut
                if !text_accumulator.is_empty() {
                    clipboard::inject_text(&text_accumulator);
                    text_accumulator.clear();
                    // Rapid-fire: reduced from 30ms to 10ms for faster fragment chaining
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                clipboard::simulate_shortcut(modifiers, *key);
                std::thread::sleep(std::time::Duration::from_millis(30));
            }
        }
    }

    // Flush any remaining text
    if !text_accumulator.is_empty() {
        if let Some(offset) = cursor_offset {
            clipboard::inject_text_with_cursor(&text_accumulator, offset);
        } else {
            clipboard::inject_text(&text_accumulator);
        }
    } else if let Some(offset) = cursor_offset {
        // Cursor needs positioning but we didn't inject text in this final flush.
        // Just move the cursor.
        if offset < 0 {
            clipboard::move_cursor_left(offset.unsigned_abs() as usize);
        }
    }
}

/// Perform the text substitution
pub async fn perform_substitution(snippet: &Snippet, ollama: &OllamaClient) {
    use crate::snippet::ContentType;
    // L2: Track substitution timing for performance metrics
    let start = Instant::now();

    log::info!("[ENGINE] perform_substitution START: keyword='{}' content_type={:?}", snippet.keyword, snippet.content_type);

    // P12: Evaluate variables and get fragments instead of flat text
    let (expanded_text, fragments, cursor_offset) = if !snippet.snippet.is_empty() {
        match variable::evaluate_variables(&snippet.snippet, ollama).await {
            Ok(result) => {
                let fragments = variable::parse_fragments(&result.text);
                // Get full text for logging (exclude placeholders)
                let full_text: String = fragments.iter().map(|f| match f {
                    SnippetFragment::Text(t) => t.clone(),
                    _ => String::new(),
                }).collect();
                (full_text, fragments, result.cursor_offset)
            }
            Err(e) => {
                eprintln!("Variable evaluation error: {}", e);
                (snippet.snippet.clone(), vec![SnippetFragment::Text(snippet.snippet.clone())], None)
            }
        }
    } else {
        (String::new(), Vec::new(), None)
    };

    // Erase the trigger keyword (backspace simulation)
    // H5 fix: use chars().count() not .len() — multi-byte chars must count as one keystroke each
    clipboard::erase_trigger(snippet.keyword.chars().count());

    // Inject content based on type
    match snippet.content_type {
        ContentType::Text => {
            // P12: Render fragments sequentially
            render_fragments(&fragments, cursor_offset);
        }
        ContentType::Image => {
            if let Some(ref b64) = snippet.image_data {
                clipboard::inject_image(b64);
            }
        }
        ContentType::Both => {
            // Inject text fragments first, then image after delay
            if !fragments.is_empty() {
                render_fragments(&fragments, cursor_offset);
            }
            if let Some(ref b64) = snippet.image_data {
                std::thread::sleep(std::time::Duration::from_millis(150));
                clipboard::inject_image(b64);
            }
        }
    }

    // Update last_used_at — use non-blocking write-behind channel (H6 fix)
    // This avoids holding the DB mutex on the hot substitution path.
    let timestamp = chrono::Utc::now().to_rfc3339();
    store::async_update_last_used(&snippet.uuid, &timestamp);

    // Log
    let preview = if expanded_text.char_count() > 50 { format!("{}...", expanded_text.truncate_chars(50)) } else { expanded_text.clone() };
    let content_desc = match snippet.content_type {
        ContentType::Text => format!("'{}'", preview),
        ContentType::Image => "[image]".to_string(),
        ContentType::Both => format!("'{}' + [image]", preview),
    };
    // L2: Log substitution timing
    log::info!("Substituted '{}' → {} (took {}ms)", snippet.keyword, content_desc, start.elapsed().as_millis());

    // Show notification (if enabled)
    if NOTIFICATIONS_ENABLED.load(Ordering::Relaxed) {
        let title = format!("⚡ {}", if snippet.name.is_empty() { &snippet.keyword } else { &snippet.name });
        let body = match snippet.content_type {
            ContentType::Text => if expanded_text.char_count() > 80 { format!("{}...", expanded_text.truncate_chars(80)) } else { expanded_text },
            ContentType::Image => "[image]".to_string(),
            ContentType::Both => "[text + image]".to_string(),
        };
        std::thread::spawn(move || {
            #[cfg(target_os = "windows")]
            {
                use std::process::Command;
                use std::os::windows::process::CommandExt;
                use base64::Engine;

                // M6 fix: Encode the notification payload as Base64 to prevent PowerShell
                // injection via crafted snippet names or content containing control characters.
                let ps_payload = format!(
                    "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null; \
                    $template = [Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02); \
                    $textNodes = $template.GetElementsByTagName('text'); \
                    $textNodes.Item(0).AppendChild($template.CreateTextNode([System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('{}')))) > $null; \
                    $textNodes.Item(1).AppendChild($template.CreateTextNode([System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String('{}')))) > $null; \
                    $toast = [Windows.UI.Notifications.ToastNotification]::new($template); \
                    [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('BeefText AI').Show($toast)",
                    base64::engine::general_purpose::STANDARD.encode(title.as_bytes()),
                    base64::engine::general_purpose::STANDARD.encode(body.as_bytes()),
                );

                let _ = Command::new("powershell")
                    .args(["-WindowStyle", "Hidden", "-Command", &ps_payload])
                    .creation_flags(0x08000000)
                    .spawn();
            }
        });
    }
}
