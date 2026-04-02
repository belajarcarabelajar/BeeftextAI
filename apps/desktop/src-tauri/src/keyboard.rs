use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use parking_lot::Mutex;
use rdev::{listen, Event, EventType, Key};

#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ToUnicodeEx, GetKeyboardLayout, GetKeyState, HKL,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

/// Shared state for the keyboard hook
pub struct KeyboardState {
    pub buffer: Arc<Mutex<String>>,
    pub enabled: Arc<AtomicBool>,
    pub running: Arc<AtomicBool>,
    pub shift_pressed: Arc<AtomicBool>,
    /// L7: Track AltGr (RightAlt) state for European keyboard layouts
    pub altgr_pressed: Arc<AtomicBool>,
    /// When false, skip processing events (used during text substitution to prevent feedback loop)
    active: Arc<AtomicBool>,
}

impl KeyboardState {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(String::new())),
            enabled: Arc::new(AtomicBool::new(true)),
            running: Arc::new(AtomicBool::new(false)),
            shift_pressed: Arc::new(AtomicBool::new(false)),
            altgr_pressed: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicBool::new(true)),
        }
    }

    /// Clear the buffer
    pub fn clear_buffer(&self) {
        self.buffer.lock().clear();
    }

    /// Get the current keyboard layout for the foreground window
    #[cfg(target_os = "windows")]
    fn get_current_layout() -> HKL {
        unsafe {
            let hwnd = GetForegroundWindow();
            let tid = GetWindowThreadProcessId(hwnd, None);
            GetKeyboardLayout(tid)
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn get_current_layout() -> usize {
        0
    }

    /// Convert a virtual key and scan code to a character using the current keyboard layout.
    /// Returns the character if successful, None otherwise.
    /// This properly handles international keyboard layouts (German, French, etc.)
    /// Respects CapsLock state (M1 fix) and AltGr state (L7 fix).
    #[cfg(target_os = "windows")]
    fn vk_to_char_layout_aware(vk: u32, scan_code: u32, shift: bool, altgr: bool) -> Option<char> {
        unsafe {
            let hkl = Self::get_current_layout();

            // Prepare keyboard state array
            let mut keyboard_state = [0u8; 256];
            if shift {
                keyboard_state[0x10] = 0x80; // VK_SHIFT
            }

            // M1: CapsLock toggle state
            if GetKeyState(0x14) & 0x01 != 0 {
                keyboard_state[0x14] = 0x01; // VK_CAPITAL toggle bit
            }

            // L7: AltGr is represented as Ctrl+Alt internally on Windows.
            // Set VK_MENU (Alt) + VK_RCONTROL to enable AltGr character mapping.
            if altgr {
                keyboard_state[0x12] = 0x80; // VK_MENU
                keyboard_state[0xA3] = 0x80; // VK_RCONTROL
            }

            let mut buf = [0u16; 8];

            let result = ToUnicodeEx(
                vk,
                scan_code,
                &keyboard_state,
                &mut buf,
                0,
                Some(hkl),
            );

            if result != 0 {
                let s = String::from_utf16_lossy(&buf[..result.unsigned_abs() as usize]);
                let ch = s.chars().next()?;
                if ch.is_control() {
                    return None;
                }
                return Some(ch);
            }

            None
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn vk_to_char_layout_aware(_vk: u32, _scan_code: u32, _shift: bool, _altgr: bool) -> Option<char> {
        None
    }

    /// Convert rdev Key to a virtual key code (VK) and scan code
    fn key_to_vk_sc(key: &Key) -> Option<(u32, u32)> {
        // Map rdev Key variants to (vk, scan_code) tuples
        // Based on USB HID usage tables and common VK mapping
        match key {
            // Letters
            Key::KeyA => Some((0x41, 0x1E)),
            Key::KeyB => Some((0x42, 0x32)),
            Key::KeyC => Some((0x43, 0x21)),
            Key::KeyD => Some((0x44, 0x23)),
            Key::KeyE => Some((0x45, 0x24)),
            Key::KeyF => Some((0x46, 0x2B)),
            Key::KeyG => Some((0x47, 0x34)),
            Key::KeyH => Some((0x48, 0x33)),
            Key::KeyI => Some((0x49, 0x43)),
            Key::KeyJ => Some((0x4A, 0x3B)),
            Key::KeyK => Some((0x4B, 0x42)),
            Key::KeyL => Some((0x4C, 0x4B)),
            Key::KeyM => Some((0x4D, 0x3A)),
            Key::KeyN => Some((0x4E, 0x31)),
            Key::KeyO => Some((0x4F, 0x44)),
            Key::KeyP => Some((0x50, 0x4D)),
            Key::KeyQ => Some((0x51, 0x14)),
            Key::KeyR => Some((0x52, 0x2C)),
            Key::KeyS => Some((0x53, 0x1B)),
            Key::KeyT => Some((0x54, 0x2D)),
            Key::KeyU => Some((0x55, 0x3C)),
            Key::KeyV => Some((0x56, 0x2A)),
            Key::KeyW => Some((0x57, 0x1D)),
            Key::KeyX => Some((0x58, 0x22)),
            Key::KeyY => Some((0x59, 0x35)),
            Key::KeyZ => Some((0x5A, 0x1A)),

            // Numbers
            Key::Num0 => Some((0x30, 0x27)),
            Key::Num1 => Some((0x31, 0x1E)),
            Key::Num2 => Some((0x32, 0x1F)),
            Key::Num3 => Some((0x33, 0x20)),
            Key::Num4 => Some((0x34, 0x21)),
            Key::Num5 => Some((0x35, 0x22)),
            Key::Num6 => Some((0x36, 0x23)),
            Key::Num7 => Some((0x37, 0x24)),
            Key::Num8 => Some((0x38, 0x25)),
            Key::Num9 => Some((0x39, 0x26)),

            // Special keys
            Key::Space => Some((0x20, 0x39)),
            Key::Return => Some((0x0D, 0x5A)),
            Key::Tab => Some((0x09, 0x0D)),
            Key::Escape => Some((0x1B, 0x76)),
            Key::Backspace => Some((0x08, 0x66)),
            Key::Delete => Some((0x2E, 0xD3)),

            // Navigation
            Key::UpArrow => Some((0x26, 0xC5)),
            Key::DownArrow => Some((0x28, 0xC7)),
            Key::LeftArrow => Some((0x25, 0xCB)),
            Key::RightArrow => Some((0x27, 0xCD)),
            Key::Home => Some((0x24, 0xC7)),
            Key::End => Some((0x23, 0xCF)),

            // Punctuation and symbols
            Key::Minus => Some((0xBD, 0x2D)),
            Key::Equal => Some((0xBB, 0x2E)),
            Key::LeftBracket => Some((0xDB, 0x3A)),
            Key::RightBracket => Some((0xDD, 0x3B)),
            Key::BackSlash => Some((0xDC, 0x56)),
            Key::SemiColon => Some((0xBA, 0x27)),
            Key::Quote => Some((0xDE, 0x28)),
            Key::BackQuote => Some((0xC0, 0x29)),
            Key::Comma => Some((0xBC, 0x33)),
            Key::Dot => Some((0xBE, 0x34)),
            Key::Slash => Some((0xBF, 0x35)),

            // Function keys
            Key::F1 => Some((0x70, 0x3B)),
            Key::F2 => Some((0x71, 0x3C)),
            Key::F3 => Some((0x72, 0x3D)),
            Key::F4 => Some((0x73, 0x3E)),
            Key::F5 => Some((0x74, 0x3F)),
            Key::F6 => Some((0x75, 0x40)),
            Key::F7 => Some((0x76, 0x41)),
            Key::F8 => Some((0x77, 0x42)),
            Key::F9 => Some((0x78, 0x43)),
            Key::F10 => Some((0x79, 0x44)),
            Key::F11 => Some((0x7A, 0x57)),
            Key::F12 => Some((0x7B, 0x58)),

            // Control keys
            Key::ControlLeft => Some((0xA2, 0x1D)),
            Key::ControlRight => Some((0xA3, 0xD2)),
            Key::ShiftLeft => Some((0xA0, 0x2A)),
            Key::ShiftRight => Some((0xA1, 0x36)),

            // Other
            Key::CapsLock => Some((0x14, 0x3A)),

            // L1: Numpad keys
            Key::KpMinus    => Some((0x6D, 0x4A)),
            Key::KpPlus     => Some((0x6B, 0x4E)),
            Key::KpMultiply => Some((0x6A, 0x37)),
            Key::KpDivide   => Some((0x6F, 0xB5)),
            Key::KpReturn   => Some((0x0D, 0x9C)),
            Key::Kp0 => Some((0x60, 0x52)),
            Key::Kp1 => Some((0x61, 0x4F)),
            Key::Kp2 => Some((0x62, 0x50)),
            Key::Kp3 => Some((0x63, 0x51)),
            Key::Kp4 => Some((0x64, 0x4B)),
            Key::Kp5 => Some((0x65, 0x4C)),
            Key::Kp6 => Some((0x66, 0x4D)),
            Key::Kp7 => Some((0x67, 0x47)),
            Key::Kp8 => Some((0x68, 0x48)),
            Key::Kp9 => Some((0x69, 0x49)),

            _ => None,
        }
    }

    /// Start listening for keyboard events in a background thread
    pub fn start_listening<F>(&self, on_trigger: F)
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        if self.running.load(Ordering::Relaxed) {
            return; // Already running
        }

        let buffer = Arc::clone(&self.buffer);
        let enabled = Arc::clone(&self.enabled);
        let running = Arc::clone(&self.running);
        let shift_pressed = Arc::clone(&self.shift_pressed);
        let altgr_pressed = Arc::clone(&self.altgr_pressed);
        let active = Arc::clone(&self.active);

        running.store(true, Ordering::Relaxed);

        // Wrap on_trigger in Arc so the retry loop can clone it each iteration (H3 fix)
        let on_trigger = Arc::new(on_trigger);

        thread::spawn(move || {
            // H3 fix: auto-retry with exponential backoff if rdev::listen fails.
            // This handles transient failures from UAC prompts, secure desktop,
            // anti-cheat software, or driver restarts.
            let mut backoff_ms: u64 = 1000;
            const MAX_BACKOFF_MS: u64 = 30_000;

            loop {
                let callback = {
                    let buffer = Arc::clone(&buffer);
                    let enabled = Arc::clone(&enabled);
                    let shift_pressed = Arc::clone(&shift_pressed);
                    let altgr_pressed = Arc::clone(&altgr_pressed);
                    let active = Arc::clone(&active);
                    let on_trigger = Arc::clone(&on_trigger); // clone Arc for this iteration
                    move |event: Event| {
                        if !enabled.load(Ordering::Relaxed) {
                            return;
                        }
                        // Skip processing during text injection to prevent feedback loop
                        if !active.load(Ordering::Relaxed) {
                            return;
                        }
                        match event.event_type {
                            EventType::KeyPress(key) => {
                                match key {
                                    Key::ShiftLeft | Key::ShiftRight => {
                                        shift_pressed.store(true, Ordering::Relaxed);
                                        return;
                                    }
                                    // L7: Track AltGr (RightAlt) state
                                    Key::Alt => {
                                        altgr_pressed.store(true, Ordering::Relaxed);
                                        return;
                                    }
                                    _ => {}
                                }

                                let mut buf = buffer.lock();
                                let is_shift = shift_pressed.load(Ordering::Relaxed);
                                let is_altgr = altgr_pressed.load(Ordering::Relaxed);

                                match key {
                                    // Backspace — remove last char
                                    Key::Backspace => {
                                        buf.pop();
                                    }
                                    // Enter, Tab, Escape, Navigation keys — combo breakers (reset buffer)
                                    Key::Return | Key::Tab | Key::Escape
                                    | Key::Home | Key::End
                                    | Key::UpArrow | Key::DownArrow | Key::LeftArrow | Key::RightArrow
                                    | Key::Insert | Key::Delete => {
                                        buf.clear();
                                    }
                                    // Every normal key press — append to buffer and check for trigger immediately
                                    _ => {
                                        // Try layout-aware conversion first using Windows API
                                        if let Some((vk, sc)) = Self::key_to_vk_sc(&key) {
                                            if let Some(ch) = Self::vk_to_char_layout_aware(vk, sc, is_shift, is_altgr) {
                                                buf.push(ch);
                                                if buf.len() > 200 {
                                                    let excess = buf.len() - 200;
                                                    buf.drain(..excess);
                                                }
                                                let current = buf.clone();
                                                drop(buf);
                                                on_trigger(current);
                                                return;
                                            }
                                        }
                                        // Fallback: try generic key_to_char for keys that might not have VK mapping
                                        if let Some(ch) = Self::key_to_char_fallback(&key, is_shift) {
                                            buf.push(ch);
                                            if buf.len() > 200 {
                                                let excess = buf.len() - 200;
                                                buf.drain(..excess);
                                            }
                                            let current = buf.clone();
                                            drop(buf);
                                            on_trigger(current);
                                        }
                                    }
                                }
                            }
                            EventType::KeyRelease(key) => {
                                match key {
                                    Key::ShiftLeft | Key::ShiftRight => {
                                        shift_pressed.store(false, Ordering::Relaxed);
                                    }
                                    // L7: Release AltGr
                                    Key::Alt => {
                                        altgr_pressed.store(false, Ordering::Relaxed);
                                    }
                                    _ => {}
                                }
                            }
                            _ => {}
                        }
                    }
                };

                if let Err(e) = listen(callback) {
                    log::error!("Keyboard hook failed: {:?}. Retrying in {}ms...", e, backoff_ms);
                    thread::sleep(std::time::Duration::from_millis(backoff_ms));
                    // Exponential backoff, capped at MAX_BACKOFF_MS
                    backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                } else {
                    // listen() returned Ok — this shouldn't happen normally,
                    // but if it does we just restart immediately.
                    backoff_ms = 1000;
                }
            }
        });
    }

    pub fn set_enabled(&self, value: bool) {
        self.enabled.store(value, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Temporarily disable event processing (used during text substitution)
    pub fn set_active(&self, value: bool) {
        self.active.store(value, Ordering::Relaxed);
    }

    /// Fallback static key-to-char mapping for keys that don't have good VK mappings
    /// or when layout-aware conversion fails. Only used as a last resort.
    fn key_to_char_fallback(key: &Key, shift: bool) -> Option<char> {
        if shift {
            match key {
                Key::KeyA => Some('A'),
                Key::KeyB => Some('B'),
                Key::KeyC => Some('C'),
                Key::KeyD => Some('D'),
                Key::KeyE => Some('E'),
                Key::KeyF => Some('F'),
                Key::KeyG => Some('G'),
                Key::KeyH => Some('H'),
                Key::KeyI => Some('I'),
                Key::KeyJ => Some('J'),
                Key::KeyK => Some('K'),
                Key::KeyL => Some('L'),
                Key::KeyM => Some('M'),
                Key::KeyN => Some('N'),
                Key::KeyO => Some('O'),
                Key::KeyP => Some('P'),
                Key::KeyQ => Some('Q'),
                Key::KeyR => Some('R'),
                Key::KeyS => Some('S'),
                Key::KeyT => Some('T'),
                Key::KeyU => Some('U'),
                Key::KeyV => Some('V'),
                Key::KeyW => Some('W'),
                Key::KeyX => Some('X'),
                Key::KeyY => Some('Y'),
                Key::KeyZ => Some('Z'),
                Key::Num0 => Some(')'),
                Key::Num1 => Some('!'),
                Key::Num2 => Some('@'),
                Key::Num3 => Some('#'),
                Key::Num4 => Some('$'),
                Key::Num5 => Some('%'),
                Key::Num6 => Some('^'),
                Key::Num7 => Some('&'),
                Key::Num8 => Some('*'),
                Key::Num9 => Some('('),
                Key::Minus => Some('_'),
                Key::Equal => Some('+'),
                Key::LeftBracket => Some('{'),
                Key::RightBracket => Some('}'),
                Key::SemiColon => Some(':'),
                Key::Quote => Some('"'),
                Key::Comma => Some('<'),
                Key::Dot => Some('>'),
                Key::Slash => Some('?'),
                Key::BackSlash => Some('|'),
                Key::BackQuote => Some('~'),
                Key::Space => Some(' '),
                _ => None,
            }
        } else {
            match key {
                Key::KeyA => Some('a'),
                Key::KeyB => Some('b'),
                Key::KeyC => Some('c'),
                Key::KeyD => Some('d'),
                Key::KeyE => Some('e'),
                Key::KeyF => Some('f'),
                Key::KeyG => Some('g'),
                Key::KeyH => Some('h'),
                Key::KeyI => Some('i'),
                Key::KeyJ => Some('j'),
                Key::KeyK => Some('k'),
                Key::KeyL => Some('l'),
                Key::KeyM => Some('m'),
                Key::KeyN => Some('n'),
                Key::KeyO => Some('o'),
                Key::KeyP => Some('p'),
                Key::KeyQ => Some('q'),
                Key::KeyR => Some('r'),
                Key::KeyS => Some('s'),
                Key::KeyT => Some('t'),
                Key::KeyU => Some('u'),
                Key::KeyV => Some('v'),
                Key::KeyW => Some('w'),
                Key::KeyX => Some('x'),
                Key::KeyY => Some('y'),
                Key::KeyZ => Some('z'),
                Key::Num0 => Some('0'),
                Key::Num1 => Some('1'),
                Key::Num2 => Some('2'),
                Key::Num3 => Some('3'),
                Key::Num4 => Some('4'),
                Key::Num5 => Some('5'),
                Key::Num6 => Some('6'),
                Key::Num7 => Some('7'),
                Key::Num8 => Some('8'),
                Key::Num9 => Some('9'),
                Key::Minus => Some('-'),
                Key::Equal => Some('='),
                Key::LeftBracket => Some('['),
                Key::RightBracket => Some(']'),
                Key::SemiColon => Some(';'),
                Key::Quote => Some('\''),
                Key::Comma => Some(','),
                Key::Dot => Some('.'),
                Key::Slash => Some('/'),
                Key::BackSlash => Some('\\'),
                Key::BackQuote => Some('`'),
                Key::Space => Some(' '),
                _ => None,
            }
        }
    }
}
