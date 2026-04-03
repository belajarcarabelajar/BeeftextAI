// clipboard.rs — Super Ultra Plan: P5, P6, P7, P8
//
// P5: Modifier key backup/restore before/after erase and paste
// P6: Configurable clipboard restore delay (default 500ms)
// P7: Native Win32 SendInput instead of rdev::simulate for key events
// P8: UTF-16 surrogate pair support in SendInput Unicode fallback

use std::thread;
use std::time::Duration;

#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT,
    KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, KEYEVENTF_EXTENDEDKEY,
    GetKeyState, VIRTUAL_KEY,
};

// ═══════════════════════════════════════════════════════════════════════════════
// P5: Modifier key backup/restore
// ═══════════════════════════════════════════════════════════════════════════════

/// VK codes for modifier keys that must be released before substitution
#[cfg(target_os = "windows")]
const MODIFIER_VKS_RELEASE: &[u16] = &[
    0xA0, 0xA1, // VK_LSHIFT, VK_RSHIFT
    0xA2, 0xA3, // VK_LCONTROL, VK_RCONTROL
    0xA4, 0xA5, // VK_LMENU, VK_RMENU
    0x5B, 0x5C, // VK_LWIN, VK_RWIN
];

/// P5: Save which modifier keys are currently pressed and release them.
/// Returns the list of VK codes that were pressed (to be restored later).
/// This mirrors original Beeftext's backupAndReleaseModifierKeys().
#[cfg(target_os = "windows")]
fn backup_and_release_modifiers() -> Vec<u16> {
    let mut pressed = Vec::new();
    unsafe {
        for &vk in MODIFIER_VKS_RELEASE {
            if GetKeyState(vk as i32) < 0 {
                pressed.push(vk);
                send_key_up_vk(vk);
            }
        }
    }
    // Small delay to let key-up events register
    if !pressed.is_empty() {
        thread::sleep(Duration::from_millis(2));
    }
    pressed
}

/// P5: Restore previously pressed modifier keys by synthesizing key-down events.
/// This mirrors original Beeftext's restoreModifierKeys().
#[cfg(target_os = "windows")]
fn restore_modifiers(keys: &[u16]) {
    for &vk in keys {
        send_key_down_vk(vk);
    }
}

#[cfg(not(target_os = "windows"))]
fn backup_and_release_modifiers() -> Vec<u16> { Vec::new() }

#[cfg(not(target_os = "windows"))]
fn restore_modifiers(_keys: &[u16]) {}

// ═══════════════════════════════════════════════════════════════════════════════
// P7: Native Win32 SendInput key simulation
// ═══════════════════════════════════════════════════════════════════════════════

/// P7: Simulate key-down using native SendInput with a virtual key code.
#[cfg(target_os = "windows")]
fn send_key_down_vk(vk: u16) {
    let mut flags = windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0);
    // Extended keys: navigation, insert, delete, numpad enter, right ctrl/alt
    if is_extended_key(vk) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

/// P7: Simulate key-up using native SendInput with a virtual key code.
#[cfg(target_os = "windows")]
fn send_key_up_vk(vk: u16) {
    let mut flags = KEYEVENTF_KEYUP;
    if is_extended_key(vk) {
        flags |= KEYEVENTF_EXTENDEDKEY;
    }
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

/// P7: Simulate key-down + key-up using native SendInput.
#[cfg(target_os = "windows")]
fn send_key_press_vk(vk: u16) {
    send_key_down_vk(vk);
    thread::sleep(Duration::from_millis(1));
    send_key_up_vk(vk);
}

/// Check if a VK is an extended key (requires KEYEVENTF_EXTENDEDKEY flag)
#[cfg(target_os = "windows")]
fn is_extended_key(vk: u16) -> bool {
    matches!(vk,
        0x21..=0x28 | // PageUp/Down, End, Home, Arrow keys
        0x2D | 0x2E | // Insert, Delete
        0x5B | 0x5C | // Win keys
        0xA3 | 0xA5 | // RControl, RMenu
        0x90          // NumLock
    )
}

// Fallback for non-Windows
#[cfg(not(target_os = "windows"))]
fn send_key_down_vk(_vk: u16) {}
#[cfg(not(target_os = "windows"))]
fn send_key_up_vk(_vk: u16) {}
#[cfg(not(target_os = "windows"))]
fn send_key_press_vk(_vk: u16) {}

// ═══════════════════════════════════════════════════════════════════════════════
// P8: SendInput Unicode character injection (with surrogate pair support)
// ═══════════════════════════════════════════════════════════════════════════════

/// P8: SendInput-based text injection using KEYEVENTF_UNICODE.
/// Supports all Unicode codepoints including emoji (via UTF-16 surrogate pairs).
#[cfg(target_os = "windows")]
fn send_input_chars_win32(text: &str) {
    for ch in text.chars() {
        let codepoint = ch as u32;
        if codepoint <= 0xFFFF {
            // BMP character — single SendInput event
            send_unicode_char(codepoint as u16);
        } else {
            // P8: Supplementary plane — encode as UTF-16 surrogate pair
            let adjusted = codepoint - 0x10000;
            let high_surrogate = ((adjusted >> 10) + 0xD800) as u16;
            let low_surrogate = ((adjusted & 0x3FF) + 0xDC00) as u16;
            send_unicode_char(high_surrogate);
            send_unicode_char(low_surrogate);
        }
        thread::sleep(Duration::from_millis(1));
    }
}

#[cfg(target_os = "windows")]
fn send_unicode_char(char_val: u16) {
    let key_down = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
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
                wVk: VIRTUAL_KEY(0),
                wScan: char_val,
                dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        SendInput(&[key_down, key_up], std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(not(target_os = "windows"))]
fn send_input_chars_win32(_text: &str) {}

// ═══════════════════════════════════════════════════════════════════════════════
// Configurable delays
// ═══════════════════════════════════════════════════════════════════════════════

/// L4: Configurable backspace delay (default 2ms)
static BACKSPACE_DELAY_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(2);

/// P6: Configurable clipboard restore delay (default 500ms, was 80ms — too fast for Electron apps)
static CLIPBOARD_RESTORE_DELAY_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(500);

/// Update the per-backspace delay
#[allow(dead_code)]
pub fn set_backspace_delay_ms(ms: u64) {
    BACKSPACE_DELAY_MS.store(ms, std::sync::atomic::Ordering::Relaxed);
}

/// P6: Update clipboard restore delay
#[allow(dead_code)]
pub fn set_clipboard_restore_delay_ms(ms: u64) {
    CLIPBOARD_RESTORE_DELAY_MS.store(ms, std::sync::atomic::Ordering::Relaxed);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Core substitution functions
// ═══════════════════════════════════════════════════════════════════════════════

/// Simulate backspace key presses to erase the trigger keyword.
/// P5: Wraps with modifier backup/restore to prevent stuck modifiers.
/// P7: Uses native SendInput instead of rdev::simulate.
pub fn erase_trigger(keyword_len: usize) {
    let pressed_mods = backup_and_release_modifiers();
    let delay = BACKSPACE_DELAY_MS.load(std::sync::atomic::Ordering::Relaxed);

    for _ in 0..keyword_len {
        send_key_press_vk(0x08); // VK_BACK
        thread::sleep(Duration::from_millis(delay));
    }
    // Post-erase settle
    thread::sleep(Duration::from_millis(15));

    restore_modifiers(&pressed_mods);
}

/// Inject text via clipboard paste (Ctrl+V).
/// P5: Releases modifiers before paste, restores after.
/// P6: Uses configurable clipboard restore delay (default 500ms).
/// P7: Uses native SendInput for Ctrl+V simulation.
pub fn inject_text(text: &str) {
    use arboard::Clipboard;

    let mut clipboard = None;
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
            log::warn!("inject_text: clipboard inaccessible, using SendInput fallback");
            let pressed_mods = backup_and_release_modifiers();
            send_input_chars_win32(text);
            restore_modifiers(&pressed_mods);
            return;
        }
    };

    // Backup current clipboard content
    let backup = clipboard.get_text().ok();

    // Set snippet text to clipboard
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
        log::warn!("inject_text: clipboard write failed, using SendInput fallback");
        let pressed_mods = backup_and_release_modifiers();
        send_input_chars_win32(text);
        restore_modifiers(&pressed_mods);
        return;
    }

    thread::sleep(Duration::from_millis(10));

    // P5: Release modifiers before paste
    let pressed_mods = backup_and_release_modifiers();

    // P7: Simulate Ctrl+V using native SendInput
    let ctrl_v_ok = {
        send_key_down_vk(0xA2); // VK_LCONTROL
        thread::sleep(Duration::from_millis(2));
        send_key_down_vk(0x56); // 'V'
        thread::sleep(Duration::from_millis(2));
        send_key_up_vk(0x56);
        thread::sleep(Duration::from_millis(2));
        send_key_up_vk(0xA2);
        true
    };

    // P5: Restore modifiers
    restore_modifiers(&pressed_mods);

    if !ctrl_v_ok {
        log::warn!("inject_text: Ctrl+V simulation failed, using SendInput fallback");
        if let Some(original) = backup {
            let _ = clipboard.set_text(&original);
        }
        let pressed_mods2 = backup_and_release_modifiers();
        send_input_chars_win32(text);
        restore_modifiers(&pressed_mods2);
        return;
    }

    // P6: Configurable clipboard restore delay (default 500ms)
    let restore_delay = CLIPBOARD_RESTORE_DELAY_MS.load(std::sync::atomic::Ordering::Relaxed);
    thread::sleep(Duration::from_millis(restore_delay));

    // Restore original clipboard
    if let Some(original) = backup {
        for _ in 0..3 {
            if clipboard.set_text(&original).is_ok() { break; }
            thread::sleep(Duration::from_millis(15));
        }
    }
}

pub fn inject_image(base64_data: &str) {
    use arboard::Clipboard;

    // Strip "data:image/...;base64," prefix if present
    let actual_b64 = if let Some(pos) = base64_data.rfind(',') {
        &base64_data[pos + 1..]
    } else {
        base64_data
    };

    let image_bytes = match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, actual_b64) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Failed to decode base64 image: {}", e);
            return;
        }
    };

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

    let backup_text = clipboard.get_text().ok();
    let backup_image: Option<arboard::ImageData> = clipboard.get_image().ok();

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

    // P5: Release modifiers before Ctrl+V
    let pressed_mods = backup_and_release_modifiers();

    // P7: Simulate Ctrl+V with native SendInput
    simulate_key_combo_native(0xA2, 0x56); // LCtrl + V

    // P5: Restore modifiers
    restore_modifiers(&pressed_mods);

    let restore_delay = CLIPBOARD_RESTORE_DELAY_MS.load(std::sync::atomic::Ordering::Relaxed);
    thread::sleep(Duration::from_millis(restore_delay));

    // Restore original clipboard
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
    inject_text(text);
    thread::sleep(Duration::from_millis(150));
    inject_image(base64_image);
}

/// Simulate pressing Left arrow key N times to move cursor left
pub fn move_cursor_left(count: usize) {
    let pressed_mods = backup_and_release_modifiers();
    for _ in 0..count {
        send_key_press_vk(0x25); // VK_LEFT
        thread::sleep(Duration::from_millis(2));
    }
    thread::sleep(Duration::from_millis(5));
    restore_modifiers(&pressed_mods);
}

/// Simulate pressing Right arrow key N times to move cursor right
pub fn move_cursor_right(count: usize) {
    let pressed_mods = backup_and_release_modifiers();
    for _ in 0..count {
        send_key_press_vk(0x27); // VK_RIGHT
        thread::sleep(Duration::from_millis(2));
    }
    thread::sleep(Duration::from_millis(5));
    restore_modifiers(&pressed_mods);
}

/// Inject text with cursor positioned at a specific offset from the end
pub fn inject_text_with_cursor(text: &str, negative_offset: i32) {
    if negative_offset == 0 {
        inject_text(text);
        return;
    }

    inject_text(text);

    let move_count = negative_offset.unsigned_abs() as usize;
    if negative_offset < 0 {
        move_cursor_left(move_count);
    } else {
        move_cursor_right(move_count);
    }
}

/// P7: Simulate a single key press and release using native SendInput.
/// This is the public API used by variable.rs for #{key:} variables.
pub fn simulate_key_press(key: rdev::Key) {
    if let Some(vk) = rdev_key_to_vk(key) {
        let pressed_mods = backup_and_release_modifiers();
        send_key_press_vk(vk);
        restore_modifiers(&pressed_mods);
    } else {
        // Fallback to rdev for keys we don't have VK mapping for
        let _ = rdev::simulate(&rdev::EventType::KeyPress(key));
        thread::sleep(Duration::from_millis(1));
        let _ = rdev::simulate(&rdev::EventType::KeyRelease(key));
    }
}

/// P7: Native key combo simulation (modifier + key)
fn simulate_key_combo_native(modifier_vk: u16, key_vk: u16) {
    send_key_down_vk(modifier_vk);
    thread::sleep(Duration::from_millis(2));
    send_key_down_vk(key_vk);
    thread::sleep(Duration::from_millis(2));
    send_key_up_vk(key_vk);
    thread::sleep(Duration::from_millis(2));
    send_key_up_vk(modifier_vk);
}

/// Simulate a key combo (modifier + key) — public API for variable.rs
pub fn simulate_key_combo(modifier: rdev::Key, key: rdev::Key) {
    let mod_vk = rdev_key_to_vk(modifier);
    let key_vk = rdev_key_to_vk(key);
    if let (Some(m), Some(k)) = (mod_vk, key_vk) {
        let pressed_mods = backup_and_release_modifiers();
        simulate_key_combo_native(m, k);
        restore_modifiers(&pressed_mods);
    } else {
        // Fallback to rdev
        let _ = rdev::simulate(&rdev::EventType::KeyPress(modifier));
        thread::sleep(Duration::from_millis(2));
        let _ = rdev::simulate(&rdev::EventType::KeyPress(key));
        thread::sleep(Duration::from_millis(2));
        let _ = rdev::simulate(&rdev::EventType::KeyRelease(key));
        thread::sleep(Duration::from_millis(2));
        let _ = rdev::simulate(&rdev::EventType::KeyRelease(modifier));
    }
}

/// Simulate a multi-modifier shortcut (e.g. Ctrl+Shift+J)
pub fn simulate_shortcut(modifiers: &[rdev::Key], key: rdev::Key) {
    let pressed_mods = backup_and_release_modifiers();

    // Press all modifiers
    for m in modifiers {
        if let Some(vk) = rdev_key_to_vk(*m) {
            send_key_down_vk(vk);
        } else {
            let _ = rdev::simulate(&rdev::EventType::KeyPress(*m));
        }
        thread::sleep(Duration::from_millis(1));
    }
    thread::sleep(Duration::from_millis(2));

    // Press key
    if let Some(vk) = rdev_key_to_vk(key) {
        send_key_press_vk(vk);
    } else {
        let _ = rdev::simulate(&rdev::EventType::KeyPress(key));
        thread::sleep(Duration::from_millis(2));
        let _ = rdev::simulate(&rdev::EventType::KeyRelease(key));
    }
    thread::sleep(Duration::from_millis(2));

    // Release modifiers in reverse
    for m in modifiers.iter().rev() {
        if let Some(vk) = rdev_key_to_vk(*m) {
            send_key_up_vk(vk);
        } else {
            let _ = rdev::simulate(&rdev::EventType::KeyRelease(*m));
        }
        thread::sleep(Duration::from_millis(1));
    }

    restore_modifiers(&pressed_mods);
}

/// Map rdev::Key to Win32 VK code for native SendInput
fn rdev_key_to_vk(key: rdev::Key) -> Option<u16> {
    match key {
        rdev::Key::Backspace => Some(0x08),
        rdev::Key::Tab => Some(0x09),
        rdev::Key::Return => Some(0x0D),
        rdev::Key::Escape => Some(0x1B),
        rdev::Key::Space => Some(0x20),
        rdev::Key::Delete => Some(0x2E),
        rdev::Key::Insert => Some(0x2D),
        rdev::Key::Home => Some(0x24),
        rdev::Key::End => Some(0x23),
        rdev::Key::PageUp => Some(0x21),
        rdev::Key::PageDown => Some(0x22),
        rdev::Key::UpArrow => Some(0x26),
        rdev::Key::DownArrow => Some(0x28),
        rdev::Key::LeftArrow => Some(0x25),
        rdev::Key::RightArrow => Some(0x27),
        rdev::Key::ShiftLeft => Some(0xA0),
        rdev::Key::ShiftRight => Some(0xA1),
        rdev::Key::ControlLeft => Some(0xA2),
        rdev::Key::ControlRight => Some(0xA3),
        rdev::Key::Alt => Some(0xA4),
        rdev::Key::MetaLeft => Some(0x5B),
        rdev::Key::CapsLock => Some(0x14),
        rdev::Key::NumLock => Some(0x90),
        rdev::Key::PrintScreen => Some(0x2C),
        rdev::Key::Pause => Some(0x13),
        rdev::Key::F1 => Some(0x70),
        rdev::Key::F2 => Some(0x71),
        rdev::Key::F3 => Some(0x72),
        rdev::Key::F4 => Some(0x73),
        rdev::Key::F5 => Some(0x74),
        rdev::Key::F6 => Some(0x75),
        rdev::Key::F7 => Some(0x76),
        rdev::Key::F8 => Some(0x77),
        rdev::Key::F9 => Some(0x78),
        rdev::Key::F10 => Some(0x79),
        rdev::Key::F11 => Some(0x7A),
        rdev::Key::F12 => Some(0x7B),
        rdev::Key::KeyA => Some(0x41),
        rdev::Key::KeyB => Some(0x42),
        rdev::Key::KeyC => Some(0x43),
        rdev::Key::KeyD => Some(0x44),
        rdev::Key::KeyE => Some(0x45),
        rdev::Key::KeyF => Some(0x46),
        rdev::Key::KeyG => Some(0x47),
        rdev::Key::KeyH => Some(0x48),
        rdev::Key::KeyI => Some(0x49),
        rdev::Key::KeyJ => Some(0x4A),
        rdev::Key::KeyK => Some(0x4B),
        rdev::Key::KeyL => Some(0x4C),
        rdev::Key::KeyM => Some(0x4D),
        rdev::Key::KeyN => Some(0x4E),
        rdev::Key::KeyO => Some(0x4F),
        rdev::Key::KeyP => Some(0x50),
        rdev::Key::KeyQ => Some(0x51),
        rdev::Key::KeyR => Some(0x52),
        rdev::Key::KeyS => Some(0x53),
        rdev::Key::KeyT => Some(0x54),
        rdev::Key::KeyU => Some(0x55),
        rdev::Key::KeyV => Some(0x56),
        rdev::Key::KeyW => Some(0x57),
        rdev::Key::KeyX => Some(0x58),
        rdev::Key::KeyY => Some(0x59),
        rdev::Key::KeyZ => Some(0x5A),
        rdev::Key::Num0 => Some(0x30),
        rdev::Key::Num1 => Some(0x31),
        rdev::Key::Num2 => Some(0x32),
        rdev::Key::Num3 => Some(0x33),
        rdev::Key::Num4 => Some(0x34),
        rdev::Key::Num5 => Some(0x35),
        rdev::Key::Num6 => Some(0x36),
        rdev::Key::Num7 => Some(0x37),
        rdev::Key::Num8 => Some(0x38),
        rdev::Key::Num9 => Some(0x39),
        _ => None,
    }
}
