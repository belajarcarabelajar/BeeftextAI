use arboard::Clipboard;
use std::thread;
use std::time::Duration;

/// Simulate backspace key presses to erase the trigger keyword
pub fn erase_trigger(keyword_len: usize) {
    // Erase exactly the keyword length
    let total = keyword_len;
    for _ in 0..total {
        simulate_key_press(rdev::Key::Backspace);
        thread::sleep(Duration::from_millis(10));
    }
    thread::sleep(Duration::from_millis(50));
}

pub fn inject_text(text: &str) {
    let mut clipboard = None;
    for _ in 0..10 {
        if let Ok(c) = Clipboard::new() {
            clipboard = Some(c);
            break;
        }
        thread::sleep(Duration::from_millis(100));
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
    let mut text_set = false;
    for _ in 0..5 {
        if clipboard.set_text(text).is_ok() {
            text_set = true;
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    if !text_set {
        eprintln!("Failed to set clipboard text after retries.");
        return;
    }

    thread::sleep(Duration::from_millis(30));

    // Simulate Ctrl+V
    simulate_key_combo(rdev::Key::ControlLeft, rdev::Key::KeyV);

    // Give target application ample time to process Ctrl+V asynchronously
    thread::sleep(Duration::from_millis(300));

    // Restore original clipboard
    if let Some(original) = backup {
        for _ in 0..5 {
            if clipboard.set_text(&original).is_ok() { break; }
            thread::sleep(Duration::from_millis(50));
        }
    }
}

/// Simulate a single key press and release
fn simulate_key_press(key: rdev::Key) {
    let _ = rdev::simulate(&rdev::EventType::KeyPress(key));
    thread::sleep(Duration::from_millis(5));
    let _ = rdev::simulate(&rdev::EventType::KeyRelease(key));
}

/// Simulate a key combo (modifier + key)
fn simulate_key_combo(modifier: rdev::Key, key: rdev::Key) {
    let _ = rdev::simulate(&rdev::EventType::KeyPress(modifier));
    thread::sleep(Duration::from_millis(10));
    let _ = rdev::simulate(&rdev::EventType::KeyPress(key));
    thread::sleep(Duration::from_millis(10));
    let _ = rdev::simulate(&rdev::EventType::KeyRelease(key));
    thread::sleep(Duration::from_millis(10));
    let _ = rdev::simulate(&rdev::EventType::KeyRelease(modifier));
}
