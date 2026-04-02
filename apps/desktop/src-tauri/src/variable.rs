use chrono::{Local, Duration as ChronoDuration};
use crate::ollama::OllamaClient;
use crate::clipboard;
use once_cell::sync::Lazy;
use regex::Regex;
use std::sync::mpsc;
use std::process::Command;

static RE_DATETIME_SHIFT: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{dateTime:([+-]\d+[ymdhsz]+):([^}]+)\}").unwrap());
static RE_DATETIME_SHIFT_PARTS: Lazy<Regex> = Lazy::new(|| Regex::new(r"([+-])(\d+)([ymdhsz])").unwrap());
static RE_DATETIME: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{dateTime:([^}]+)\}").unwrap());
static RE_DATE: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{date:([^}]+)\}").unwrap());
static RE_TIME: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{time:([^}]+)\}").unwrap());
static RE_ENV: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{envVar:([^}]+)\}").unwrap());
static RE_UPPER: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{upper:([^}]+)\}").unwrap());
static RE_LOWER: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{lower:([^}]+)\}").unwrap());
static RE_TRIM: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{trim:([^}]+)\}").unwrap());
static RE_AI: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{ai:([^}]+)\}").unwrap());
static RE_COMBO: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{combo:([^}]+)\}").unwrap());
static RE_INPUT: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{input:([^}]*)\}").unwrap());
static RE_POWERSHELL: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{powershell:([^:}]+)(?::(\d+))?\}").unwrap());
static RE_KEY: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{key:([^:}]+)(?::(\d+))?\}").unwrap());
static RE_SHORTCUT: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{shortcut:([^}]+)\}").unwrap());
static RE_DELAY: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{delay:(\d+)\}").unwrap());
static RE_CURSOR: Lazy<Regex> = Lazy::new(|| Regex::new(r"#\{cursor\}").unwrap());

/// Result of evaluating template variables
pub struct ExpandedText {
    pub text: String,
    /// Cursor offset from end of text. Negative = move left from end, Positive = move right from end.
    /// None means no cursor marker was present.
    pub cursor_offset: Option<i32>,
    /// Delays to execute (in milliseconds), in order they appear in the text
    pub delays: Vec<u64>,
}

/// Map a key name string to rdev::Key
/// Key names follow the Beeftext wiki convention but are mapped to rdev::Key variants.
/// Supported names: space, tab, enter, insert, delete, home, end, pageup, pagedown,
/// up, down, left, right, escape/esc, printscreen, pause, numlock, backspace,
/// windows/win/meta, control/ctrl, alt, shift, f1..f12, a-z, 0-9
fn key_name_to_rdev(name: &str) -> Option<rdev::Key> {
    match name.to_lowercase().as_str() {
        "space"    => Some(rdev::Key::Space),
        "tab"      => Some(rdev::Key::Tab),
        "enter"    => Some(rdev::Key::Return),
        "insert"   => Some(rdev::Key::Insert),
        "delete"   => Some(rdev::Key::Delete),
        "home"     => Some(rdev::Key::Home),
        "end"      => Some(rdev::Key::End),
        "pageup"   => Some(rdev::Key::PageUp),
        "pagedown" => Some(rdev::Key::PageDown),
        "up"       => Some(rdev::Key::UpArrow),
        "down"     => Some(rdev::Key::DownArrow),
        "left"     => Some(rdev::Key::LeftArrow),
        "right"    => Some(rdev::Key::RightArrow),
        "escape" | "esc" => Some(rdev::Key::Escape),
        "printscreen" | "print_screen" => Some(rdev::Key::PrintScreen),
        "pause"    => Some(rdev::Key::Pause),
        "numlock"  | "num_lock" => Some(rdev::Key::NumLock),
        "backspace" => Some(rdev::Key::Backspace),
        "windows" | "win" | "meta" => Some(rdev::Key::MetaLeft),
        "control" | "ctrl" => Some(rdev::Key::ControlLeft),
        "alt"     => Some(rdev::Key::Alt),
        "shift"   => Some(rdev::Key::ShiftLeft),
        "f1"  => Some(rdev::Key::F1),
        "f2"  => Some(rdev::Key::F2),
        "f3"  => Some(rdev::Key::F3),
        "f4"  => Some(rdev::Key::F4),
        "f5"  => Some(rdev::Key::F5),
        "f6"  => Some(rdev::Key::F6),
        "f7"  => Some(rdev::Key::F7),
        "f8"  => Some(rdev::Key::F8),
        "f9"  => Some(rdev::Key::F9),
        "f10" => Some(rdev::Key::F10),
        "f11" => Some(rdev::Key::F11),
        "f12" => Some(rdev::Key::F12),
        // Letter keys
        "a" => Some(rdev::Key::KeyA),
        "b" => Some(rdev::Key::KeyB),
        "c" => Some(rdev::Key::KeyC),
        "d" => Some(rdev::Key::KeyD),
        "e" => Some(rdev::Key::KeyE),
        "f" => Some(rdev::Key::KeyF),
        "g" => Some(rdev::Key::KeyG),
        "h" => Some(rdev::Key::KeyH),
        "i" => Some(rdev::Key::KeyI),
        "j" => Some(rdev::Key::KeyJ),
        "k" => Some(rdev::Key::KeyK),
        "l" => Some(rdev::Key::KeyL),
        "m" => Some(rdev::Key::KeyM),
        "n" => Some(rdev::Key::KeyN),
        "o" => Some(rdev::Key::KeyO),
        "p" => Some(rdev::Key::KeyP),
        "q" => Some(rdev::Key::KeyQ),
        "r" => Some(rdev::Key::KeyR),
        "s" => Some(rdev::Key::KeyS),
        "t" => Some(rdev::Key::KeyT),
        "u" => Some(rdev::Key::KeyU),
        "v" => Some(rdev::Key::KeyV),
        "w" => Some(rdev::Key::KeyW),
        "x" => Some(rdev::Key::KeyX),
        "y" => Some(rdev::Key::KeyY),
        "z" => Some(rdev::Key::KeyZ),
        // Number keys
        "0" => Some(rdev::Key::Num0),
        "1" => Some(rdev::Key::Num1),
        "2" => Some(rdev::Key::Num2),
        "3" => Some(rdev::Key::Num3),
        "4" => Some(rdev::Key::Num4),
        "5" => Some(rdev::Key::Num5),
        "6" => Some(rdev::Key::Num6),
        "7" => Some(rdev::Key::Num7),
        "8" => Some(rdev::Key::Num8),
        "9" => Some(rdev::Key::Num9),
        _ => None,
    }
}

/// Parse a shortcut string like "Ctrl+Shift+J" into modifiers and key
fn parse_shortcut(shortcut: &str) -> Option<(Vec<rdev::Key>, rdev::Key)> {
    let parts: Vec<&str> = shortcut.split('+').collect();
    if parts.is_empty() {
        return None;
    }
    let key_part = parts.last()?;
    let key = key_name_to_rdev(key_part)?;
    let mut modifiers = Vec::new();
    for part in &parts[..parts.len() - 1] {
        if let Some(m) = key_name_to_rdev(part) {
            modifiers.push(m);
        }
    }
    Some((modifiers, key))
}

/// Show an interactive input dialog and return the user's input (blocking)
#[cfg(target_os = "windows")]
fn show_input_dialog_blocking(desc: &str) -> String {
    use std::os::windows::process::CommandExt;
    let desc_escaped = desc.replace("'", "''");
    let ps = format!(
        "Add-Type -AssemblyName Microsoft.VisualBasic; \
         [Microsoft.VisualBasic.Interaction]::InputBox('{}', 'BeefText AI Input', '')",
        desc_escaped
    );
    let output = Command::new("powershell")
        .args(["-WindowStyle", "Hidden", "-Command", &ps])
        .creation_flags(0x08000000)
        .output();
    output
        .map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_string())
        .unwrap_or_default()
}

#[cfg(not(target_os = "windows"))]
fn show_input_dialog_blocking(_desc: &str) -> String {
    String::new()
}

/// Execute a PowerShell script and return its stdout (blocking)
#[cfg(target_os = "windows")]
fn run_powershell_script_blocking(path: &str, timeout_ms: u64) -> String {
    use std::os::windows::process::CommandExt;
    let ps = format!("& '{}'", path);
    if timeout_ms == 0 {
        // Indefinite wait — use output() directly
        let output = Command::new("powershell")
            .args(["-WindowStyle", "Hidden", "-Command", &ps])
            .creation_flags(0x08000000)
            .output();
        return output
            .map(|o| String::from_utf8_lossy(&o.stdout).trim_end().to_string())
            .unwrap_or_default();
    }
    // Timed execution — spawn thread so we can kill after timeout
    let (tx, rx) = mpsc::channel();
    let path_owned = path.to_string();
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            use std::io::Read;
            let ps_inner = format!("& '{}'", path_owned);
            let child = Command::new("powershell")
                .args(["-WindowStyle", "Hidden", "-Command", &ps_inner])
                .creation_flags(0x08000000)
                .spawn();
            if let Ok(mut child) = child {
                std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
                let _ = child.kill();
                let mut output_buf = Vec::new();
                if let Some(ref mut stdout) = child.stdout {
                    let _ = stdout.read_to_end(&mut output_buf);
                }
                let output_str = String::from_utf8_lossy(&output_buf).trim_end().to_string();
                let _ = tx.send(output_str);
            } else {
                let _ = tx.send(String::new());
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = tx.send(String::new());
        }
    });
    rx.recv().unwrap_or_default()
}

#[cfg(not(target_os = "windows"))]
fn run_powershell_script_blocking(_path: &str, _timeout_ms: u64) -> String {
    String::new()
}

/// Evaluate all template variables in a snippet text
/// Variables: #{clipboard}, #{date}, #{time}, #{dateTime:format}, #{dateTime:+offset:format}, #{input:desc},
///            #{combo:keyword}, #{envVar:name}, #{ai:prompt},
///            #{upper:text}, #{lower:text}, #{trim:text}, #{cursor},
///            #{input:desc}, #{powershell:path}, #{powershell:path:timeoutMs},
///            #{key:keyname}, #{key:keyname:count}, #{shortcut:mod+key}, #{delay:ms}
/// depth: recursion depth for #{combo:}, max 5 (M3 fix)
pub async fn evaluate_variables(text: &str, ollama: &OllamaClient) -> Result<ExpandedText, String> {
    evaluate_variables_inner(text, ollama, 0).await
}

fn evaluate_variables_inner<'a>(
    text: &'a str,
    ollama: &'a OllamaClient,
    depth: usize,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<ExpandedText, String>> + Send + 'a>> {
    Box::pin(async move {
    let mut result = text.to_string();
    let mut delays = Vec::new();

    // #{clipboard} — current clipboard content
    if result.contains("#{clipboard}") {
        let clip_text = arboard::Clipboard::new()
            .and_then(|mut c| c.get_text())
            .unwrap_or_default();
        result = result.replace("#{clipboard}", &clip_text);
    }

    // #{date} — current date (default format)
    result = result.replace("#{date}", &Local::now().format("%Y-%m-%d").to_string());

    // #{time} — current time (default format)
    result = result.replace("#{time}", &Local::now().format("%H:%M:%S").to_string());

    // #{dateTime:+offset:format} — date/time with shift (e.g. +1d-2h) BEFORE plain dateTime:format
    {
        let dt_result = result.clone();
        for cap in RE_DATETIME_SHIFT.captures_iter(&dt_result) {
            let full_match = &cap[0];
            let shift_str = &cap[1];
            let format = &cap[2];
            let mut offset = ChronoDuration::zero();
            for part_cap in RE_DATETIME_SHIFT_PARTS.captures_iter(shift_str) {
                let sign = if &part_cap[1] == "-" { -1 } else { 1 };
                let value: i64 = part_cap[2].parse().unwrap_or(0);
                let unit = &part_cap[3];
                let delta = match unit {
                    "y" => ChronoDuration::days(365 * value as i64),
                    "M" => ChronoDuration::days(30 * value as i64),
                    "w" => ChronoDuration::weeks(value as i64),
                    "d" => ChronoDuration::days(value as i64),
                    "h" => ChronoDuration::hours(value as i64),
                    "m" => ChronoDuration::minutes(value as i64),
                    "s" => ChronoDuration::seconds(value as i64),
                    "z" => ChronoDuration::milliseconds(value as i64),
                    _ => ChronoDuration::zero(),
                };
                offset = if sign < 0 { offset - delta } else { offset + delta };
            }
            let new_time = Local::now() + offset;
            let formatted = new_time.format(format).to_string();
            result = result.replace(full_match, &formatted);
        }
    }

    // #{dateTime:format} — custom date/time format (plain, no shift)
    let dt_result = result.clone();
    for cap in RE_DATETIME.captures_iter(&dt_result) {
        let full_match = &cap[0];
        // Skip if this is actually a shift variant (already handled above)
        if full_match.starts_with("#{dateTime:+") || full_match.starts_with("#{dateTime:-") {
            continue;
        }
        let format = &cap[1];
        let formatted = Local::now().format(format).to_string();
        result = result.replace(full_match, &formatted);
    }

    // #{date:format} — custom date format
    let date_result = result.clone();
    for cap in RE_DATE.captures_iter(&date_result) {
        let full_match = &cap[0];
        let format = &cap[1];
        let formatted = Local::now().format(format).to_string();
        result = result.replace(full_match, &formatted);
    }

    // #{time:format} — custom time format
    let time_result = result.clone();
    for cap in RE_TIME.captures_iter(&time_result) {
        let full_match = &cap[0];
        let format = &cap[1];
        let formatted = Local::now().format(format).to_string();
        result = result.replace(full_match, &formatted);
    }

    // #{envVar:name} — environment variable
    let env_result = result.clone();
    for cap in RE_ENV.captures_iter(&env_result) {
        let full_match = &cap[0];
        let var_name = &cap[1];
        let value = std::env::var(var_name).unwrap_or_default();
        result = result.replace(full_match, &value);
    }

    // #{upper:text} — uppercase
    let upper_result = result.clone();
    for cap in RE_UPPER.captures_iter(&upper_result) {
        let full_match = &cap[0];
        let text_val = &cap[1];
        result = result.replace(full_match, &text_val.to_uppercase());
    }

    // #{lower:text} — lowercase
    let lower_result = result.clone();
    for cap in RE_LOWER.captures_iter(&lower_result) {
        let full_match = &cap[0];
        let text_val = &cap[1];
        result = result.replace(full_match, &text_val.to_lowercase());
    }

    // #{trim:text} — trim whitespace
    let trim_result = result.clone();
    for cap in RE_TRIM.captures_iter(&trim_result) {
        let full_match = &cap[0];
        let text_val = &cap[1];
        result = result.replace(full_match, text_val.trim());
    }

    // #{ai:prompt} — generate text via Ollama (with 30s timeout — L3 fix)
    let ai_result = result.clone();
    for cap in RE_AI.captures_iter(&ai_result) {
        let full_match = &cap[0];
        let prompt = &cap[1];
        let gen_future = ollama.generate(prompt, None);
        match tokio::time::timeout(std::time::Duration::from_secs(30), gen_future).await {
            Ok(Ok(ai_text)) => {
                result = result.replace(full_match, &ai_text);
            }
            Ok(Err(e)) => {
                eprintln!("AI variable error: {}", e);
                result = result.replace(full_match, &format!("[AI Error: {}]", e));
            }
            Err(_timeout) => {
                log::error!("#{{ai:prompt}} timed out after 30s");
                result = result.replace(full_match, "[AI Error: timeout]");
            }
        }
    }

    // #{combo:keyword} — reference another snippet, expanding its variables recursively (M3 fix)
    let combo_result = result.clone();
    for cap in RE_COMBO.captures_iter(&combo_result) {
        let full_match = &cap[0];
        let keyword = &cap[1];
        if let Ok(snippets) = crate::store::get_all_snippets() {
            if let Some(referenced) = snippets.iter().find(|s| s.keyword == keyword) {
                if depth < 5 {
                    // Recursively expand variables inside the combo snippet (depth-limited)
                    match evaluate_variables_inner(&referenced.snippet, ollama, depth + 1).await {
                        Ok(expanded) => result = result.replace(full_match, &expanded.text),
                        Err(_) => result = result.replace(full_match, &referenced.snippet),
                    }
                } else {
                    // Max depth reached — substitute raw text to avoid infinite recursion
                    result = result.replace(full_match, &referenced.snippet);
                }
            }
        }
    }

    // #{input:description} — interactive input dialog (blocking via PowerShell)
    {
        let input_result = result.clone();
        for cap in RE_INPUT.captures_iter(&input_result) {
            let full_match = cap[0].to_string();
            let desc = cap[1].to_string();
            let input_value = tokio::task::spawn_blocking(move || {
                show_input_dialog_blocking(&desc)
            }).await.unwrap_or_default();
            result = result.replace(&full_match, &input_value);
        }
    }

    // #{powershell:path} and #{powershell:path:timeoutMs}
    {
        let ps_result = result.clone();
        for cap in RE_POWERSHELL.captures_iter(&ps_result) {
            let full_match = cap[0].to_string();
            let path = cap[1].to_string();
            let timeout_ms: u64 = cap.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(10000);
            
            // Validate the path against command injection
            let p = std::path::Path::new(&path);
            if !p.is_absolute() || p.extension().and_then(|s| s.to_str()) != Some("ps1") || !p.exists() {
                result = result.replace(&full_match, "[Error: Only absolute paths to valid .ps1 files are allowed]");
                continue;
            }

            let ps_output = tokio::task::spawn_blocking(move || {
                run_powershell_script_blocking(&path, timeout_ms)
            }).await.unwrap_or_default();
            result = result.replace(&full_match, &ps_output);
        }
    }

    // #{key:keyname} and #{key:keyname:count}
    {
        let key_result = result.clone();
        for cap in RE_KEY.captures_iter(&key_result) {
            let full_match = cap[0].to_string();
            let key_name = &cap[1];
            let count: usize = cap.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(1);
            if let Some(key) = key_name_to_rdev(key_name) {
                // Simulate key presses immediately during variable evaluation
                // (These happen during expansion, keyboard hook is already disabled)
                for _ in 0..count {
                    clipboard::simulate_key_press(key);
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            result = result.replace(&full_match, "");
        }
    }

    // #{shortcut:mod+key} — e.g. #{shortcut:Ctrl+Shift+J}
    {
        let shortcut_result = result.clone();
        for cap in RE_SHORTCUT.captures_iter(&shortcut_result) {
            let full_match = cap[0].to_string();
            let shortcut_str = &cap[1];
            if let Some((modifiers, key)) = parse_shortcut(shortcut_str) {
                clipboard::simulate_shortcut(&modifiers, key);
            }
            result = result.replace(&full_match, "");
        }
    }

    // #{delay:ms} — collect delays to execute later (not evaluated here)
    {
        let delay_result = result.clone();
        for cap in RE_DELAY.captures_iter(&delay_result) {
            let full_match = cap[0].to_string();
            if let Ok(ms) = cap[1].parse::<u64>() {
                delays.push(ms);
            }
            result = result.replace(&full_match, "");
        }
    }

    // #{cursor} — cursor position marker
    let cursor_count = RE_CURSOR.find_iter(&result).count();
    if cursor_count > 1 {
        return Err("Only one #{cursor} marker is allowed".to_string());
    }

    let cursor_offset = if cursor_count == 1 {
        if let Some(pos) = result.find("#{cursor}") {
            let after = &result[pos + 9..]; // 9 = len of "#{cursor}"
            let char_count = after.chars().count() as i32;
            result = result.replace("#{cursor}", "");
            Some(-char_count)
        } else {
            None
        }
    } else {
        None
    };

    Ok(ExpandedText { text: result, cursor_offset, delays })
    }) // closes Box::pin(async move {
}
