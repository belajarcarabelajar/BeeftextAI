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

/// Simulate a single key press and release
fn simulate_key_press(key: rdev::Key) {
    let _ = rdev::simulate(&rdev::EventType::KeyPress(key));
    // Reduced from 5ms to 1ms
    thread::sleep(Duration::from_millis(1));
    let _ = rdev::simulate(&rdev::EventType::KeyRelease(key));
}

/// Simulate a key combo (modifier + key)
fn simulate_key_combo(modifier: rdev::Key, key: rdev::Key) {
    let _ = rdev::simulate(&rdev::EventType::KeyPress(modifier));
    // Reduced from 10ms to 2ms
    thread::sleep(Duration::from_millis(2));
    let _ = rdev::simulate(&rdev::EventType::KeyPress(key));
    // Reduced from 10ms to 2ms
    thread::sleep(Duration::from_millis(2));
    let _ = rdev::simulate(&rdev::EventType::KeyRelease(key));
    // Reduced from 10ms to 2ms
    thread::sleep(Duration::from_millis(2));
    let _ = rdev::simulate(&rdev::EventType::KeyRelease(modifier));
}
