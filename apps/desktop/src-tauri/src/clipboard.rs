use arboard::Clipboard;
use std::thread;
use std::time::Duration;

/// SendInput-based fallback for injecting text in elevated/secured contexts.
/// Uses Win32 SendInput with KEYEVENTF_UNICODE to type each character individually.
/// This works in contexts where Ctrl+V simulation is blocked (UAC dialogs, RDP, elevated apps).
#[cfg(target_os = "windows")]
fn send_input_chars_win32(text: &str) {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_UNICODE,
    };

    for ch in text.chars() {
        let char_val = ch as u16;
        // Some chars require surrogate pairs (codepoints > 0xFFFF)
        // For simplicity we skip chars that can't be represented in UTF-16 directly
        if ch as u32 > 0xFFFF {
            continue;
        }
        let key_down = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0),
                    wScan: char_val,
                    dwFlags: KEYEVENTF_UNICODE,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        let key_up = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY(0),
                    wScan: char_val,
                    dwFlags: KEYEVENTF_UNICODE
                        | windows::Win32::UI::Input::KeyboardAndMouse::KEYEVENTF_KEYUP,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        unsafe {
            SendInput(&[key_down, key_up], std::mem::size_of::<INPUT>() as i32);
        }
        // Small delay between characters to prevent dropped inputs in slow apps
        thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(not(target_os = "windows"))]
fn send_input_chars_win32(_text: &str) {
    // No-op on non-Windows platforms
}

/// L4: Configurable backspace delay (default 2ms). Can be updated via set_backspace_delay_ms().
static BACKSPACE_DELAY_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(2);

/// Update the per-backspace delay (L4 fix). Called from settings when user changes the preference.
#[allow(dead_code)]
pub fn set_backspace_delay_ms(ms: u64) {
    BACKSPACE_DELAY_MS.store(ms, std::sync::atomic::Ordering::Relaxed);
}

/// Simulate backspace key presses to erase the trigger keyword
pub fn erase_trigger(keyword_len: usize) {
    let delay = BACKSPACE_DELAY_MS.load(std::sync::atomic::Ordering::Relaxed);
    for _ in 0..keyword_len {
        simulate_key_press(rdev::Key::Backspace);
        thread::sleep(Duration::from_millis(delay));
    }
    // Post-erase settle: enough for the last backspace to register in slow apps
    thread::sleep(Duration::from_millis(15));
}

pub fn inject_text(text: &str) {
    let mut clipboard = None;
    // M2 fix: exponential backoff (30, 60, 120, 240ms) for apps holding clipboard lock longer
    let mut wait_ms = 30u64;
    for _ in 0..4 {
        if let Ok(c) = Clipboard::new() {
            clipboard = Some(c);
            break;
        }
        thread::sleep(Duration::from_millis(wait_ms));
        wait_ms = (wait_ms * 2).min(240);
    }
    let mut clipboard = match clipboard {
        Some(c) => c,
        None => {
            // Clipboard unavailable — fall back to SendInput character typing
            log::warn!("inject_text: clipboard inaccessible, using SendInput fallback");
            send_input_chars_win32(text);
            return;
        }
    };

    // Backup current clipboard content
    let backup = clipboard.get_text().ok();

    // M2 fix: exponential backoff for clipboard set_text
    let mut text_set = false;
    let mut wait_ms = 15u64;
    for _ in 0..4 {
        if clipboard.set_text(text).is_ok() {
            text_set = true;
            break;
        }
        thread::sleep(Duration::from_millis(wait_ms));
        wait_ms = (wait_ms * 2).min(120);
    }

    if !text_set {
        // Clipboard write failed — fall back to SendInput
        log::warn!("inject_text: clipboard write failed, using SendInput fallback");
        send_input_chars_win32(text);
        return;
    }

    // Reduced from 30ms to 10ms
    thread::sleep(Duration::from_millis(10));

    // Simulate Ctrl+V
    let ctrl_v_ok = rdev::simulate(&rdev::EventType::KeyPress(rdev::Key::ControlLeft)).is_ok()
        && rdev::simulate(&rdev::EventType::KeyPress(rdev::Key::KeyV)).is_ok();
    thread::sleep(Duration::from_millis(2));
    let _ = rdev::simulate(&rdev::EventType::KeyRelease(rdev::Key::KeyV));
    thread::sleep(Duration::from_millis(2));
    let _ = rdev::simulate(&rdev::EventType::KeyRelease(rdev::Key::ControlLeft));

    if !ctrl_v_ok {
        // Ctrl+V simulation itself failed — try SendInput as secondary fallback
        log::warn!("inject_text: Ctrl+V simulation failed, using SendInput fallback");
        // Restore clipboard first
        if let Some(original) = backup {
            let _ = clipboard.set_text(&original);
        }
        send_input_chars_win32(text);
        return;
    }

    // Reduced from 300ms to 80ms — most apps process paste within 50-100ms
    thread::sleep(Duration::from_millis(80));

    // Restore original clipboard
    if let Some(original) = backup {
        // Reduced from 5 retries at 50ms each to 3 retries at 15ms each
        for _ in 0..3 {
            if clipboard.set_text(&original).is_ok() { break; }
            thread::sleep(Duration::from_millis(15));
        }
    }
}

pub fn inject_image(base64_data: &str) {
    // Strip "data:image/...;base64," prefix if present (from FileReader.readAsDataURL)
    let actual_b64 = if let Some(pos) = base64_data.rfind(',') {
        &base64_data[pos + 1..]
    } else {
        base64_data
    };

    // Decode base64 to raw bytes
    let image_bytes = match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, actual_b64) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Failed to decode base64 image: {}", e);
            return;
        }
    };

    // Load image and convert to RGBA
    let img = match image::load_from_memory(&image_bytes) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Failed to load image from memory: {}", e);
            return;
        }
    };

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    let img_data = arboard::ImageData {
        width: width as usize,
        height: height as usize,
        bytes: rgba.into_raw().into(),
    };

    let mut clipboard = None;
    for _ in 0..3 {
        if let Ok(c) = Clipboard::new() {
            clipboard = Some(c);
            break;
        }
        thread::sleep(Duration::from_millis(30));
    }
    let mut clipboard = match clipboard {
        Some(c) => c,
        None => {
            eprintln!("Failed to access clipboard for image after retries.");
            return;
        }
    };

    // Backup current clipboard content (try text first)
    let backup_text = clipboard.get_text().ok();
    // Also try to get image backup
    let backup_image: Option<arboard::ImageData> = clipboard.get_image().ok();

    // Set the image (with retry)
    let mut image_set = false;
    for _ in 0..3 {
        if clipboard.set_image(img_data.clone()).is_ok() {
            image_set = true;
            break;
        }
        thread::sleep(Duration::from_millis(15));
    }

    if !image_set {
        eprintln!("Failed to set clipboard image after retries.");
        return;
    }

    thread::sleep(Duration::from_millis(10));

    // Simulate Ctrl+V
    simulate_key_combo(rdev::Key::ControlLeft, rdev::Key::KeyV);

    thread::sleep(Duration::from_millis(80));

    // Restore original clipboard
    // Try to restore as image if we had one, otherwise as text
    if let Some(orig_img) = backup_image {
        let img_clone = arboard::ImageData {
            width: orig_img.width,
            height: orig_img.height,
            bytes: orig_img.bytes.clone(),
        };
        for _ in 0..3 {
            if clipboard.set_image(img_clone.clone()).is_ok() { break; }
            thread::sleep(Duration::from_millis(15));
        }
    } else if let Some(original) = backup_text {
        for _ in 0..3 {
            if clipboard.set_text(&original).is_ok() { break; }
            thread::sleep(Duration::from_millis(15));
        }
    }
}

/// Inject both text and image sequentially (for ContentType::Both)
pub fn inject_both(text: &str, base64_image: &str) {
    // First: inject text
    inject_text(text);
    // Delay to let target app finish processing the text paste
    thread::sleep(Duration::from_millis(150));
    // Second: inject image
    inject_image(base64_image);
}

/// Simulate pressing Left arrow key N times to move cursor left
pub fn move_cursor_left(count: usize) {
    for _ in 0..count {
        simulate_key_press(rdev::Key::LeftArrow);
        thread::sleep(Duration::from_millis(2));
    }
    thread::sleep(Duration::from_millis(5));
}

/// Simulate pressing Right arrow key N times to move cursor right
pub fn move_cursor_right(count: usize) {
    for _ in 0..count {
        simulate_key_press(rdev::Key::RightArrow);
        thread::sleep(Duration::from_millis(2));
    }
    thread::sleep(Duration::from_millis(5));
}

/// Inject text with cursor positioned at a specific offset from the end.
/// negative_offset: move cursor left from end (e.g., -6 means move 6 chars left from end)
pub fn inject_text_with_cursor(text: &str, negative_offset: i32) {
    if negative_offset == 0 {
        inject_text(text);
        return;
    }

    // Phase 1: Paste the full text
    inject_text(text);

    // Phase 2: Move cursor to desired position
    let move_count = negative_offset.unsigned_abs() as usize;
    if negative_offset < 0 {
        move_cursor_left(move_count);
    } else {
        move_cursor_right(move_count);
    }
}

/// Simulate a single key press and release
pub fn simulate_key_press(key: rdev::Key) {
    let _ = rdev::simulate(&rdev::EventType::KeyPress(key));
    thread::sleep(Duration::from_millis(1));
    let _ = rdev::simulate(&rdev::EventType::KeyRelease(key));
}

/// Simulate a key combo (modifier + key)
pub fn simulate_key_combo(modifier: rdev::Key, key: rdev::Key) {
    let _ = rdev::simulate(&rdev::EventType::KeyPress(modifier));
    thread::sleep(Duration::from_millis(2));
    let _ = rdev::simulate(&rdev::EventType::KeyPress(key));
    thread::sleep(Duration::from_millis(2));
    let _ = rdev::simulate(&rdev::EventType::KeyRelease(key));
    thread::sleep(Duration::from_millis(2));
    let _ = rdev::simulate(&rdev::EventType::KeyRelease(modifier));
}

/// Simulate a multi-modifier shortcut (e.g. Ctrl+Shift+J)
/// Presses all modifiers simultaneously, then the key, then releases in reverse order
pub fn simulate_shortcut(modifiers: &[rdev::Key], key: rdev::Key) {
    // Press all modifiers
    for m in modifiers {
        let _ = rdev::simulate(&rdev::EventType::KeyPress(*m));
        thread::sleep(Duration::from_millis(1));
    }
    thread::sleep(Duration::from_millis(2));
    // Press key
    let _ = rdev::simulate(&rdev::EventType::KeyPress(key));
    thread::sleep(Duration::from_millis(2));
    // Release key
    let _ = rdev::simulate(&rdev::EventType::KeyRelease(key));
    thread::sleep(Duration::from_millis(2));
    // Release modifiers in reverse order
    for m in modifiers.iter().rev() {
        let _ = rdev::simulate(&rdev::EventType::KeyRelease(*m));
        thread::sleep(Duration::from_millis(1));
    }
}
