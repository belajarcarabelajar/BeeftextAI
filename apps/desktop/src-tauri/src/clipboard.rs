use arboard::Clipboard;
use std::thread;
use std::time::Duration;

/// Simulate backspace key presses to erase the trigger keyword
pub fn erase_trigger(keyword_len: usize) {
    let total = keyword_len;
    for _ in 0..total {
        simulate_key_press(rdev::Key::Backspace);
        // Reduced from 10ms to 2ms — most apps process backspace instantly
        thread::sleep(Duration::from_millis(2));
    }
    // Reduced from 50ms to 15ms — just enough for the last backspace to register
    thread::sleep(Duration::from_millis(15));
}

pub fn inject_text(text: &str) {
    let mut clipboard = None;
    // Reduced from 10 retries at 100ms each to 3 retries at 30ms each
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
            eprintln!("Failed to access clipboard after retries.");
            return;
        }
    };

    // Backup current clipboard content
    let backup = clipboard.get_text().ok();

    // Set the snippet text (with retry)
    // Reduced from 5 retries at 50ms each to 3 retries at 15ms each
    let mut text_set = false;
    for _ in 0..3 {
        if clipboard.set_text(text).is_ok() {
            text_set = true;
            break;
        }
        thread::sleep(Duration::from_millis(15));
    }

    if !text_set {
        eprintln!("Failed to set clipboard text after retries.");
        return;
    }

    // Reduced from 30ms to 10ms
    thread::sleep(Duration::from_millis(10));

    // Simulate Ctrl+V
    simulate_key_combo(rdev::Key::ControlLeft, rdev::Key::KeyV);

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
