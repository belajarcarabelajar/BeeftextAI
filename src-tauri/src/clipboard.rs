use arboard::Clipboard;
use std::thread;
use std::time::Duration;

/// Simulate backspace key presses to erase the trigger keyword
pub fn erase_trigger(keyword_len: usize) {
    // Erase keyword + trailing space
    let total = keyword_len + 1;
    for _ in 0..total {
        simulate_key_press(rdev::Key::Backspace);
        thread::sleep(Duration::from_millis(10));
    }
    thread::sleep(Duration::from_millis(50));
}

/// Inject text by:
/// 1. Backup current clipboard
/// 2. Set snippet text to clipboard
/// 3. Simulate Ctrl+V paste
/// 4. Restore original clipboard
pub fn inject_text(text: &str) {
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to access clipboard: {}", e);
            return;
        }
    };

    // Backup current clipboard content
    let backup = clipboard.get_text().ok();

    // Set the snippet text
    if let Err(e) = clipboard.set_text(text) {
        eprintln!("Failed to set clipboard: {}", e);
        return;
    }

    thread::sleep(Duration::from_millis(30));

    // Simulate Ctrl+V
    simulate_key_combo(rdev::Key::ControlLeft, rdev::Key::KeyV);

    thread::sleep(Duration::from_millis(100));

    // Restore original clipboard
    if let Some(original) = backup {
        thread::sleep(Duration::from_millis(200));
        let _ = clipboard.set_text(&original);
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
