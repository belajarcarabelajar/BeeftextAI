use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use parking_lot::Mutex;
use rdev::{listen, Event, EventType, Key};

/// Shared state for the keyboard hook
pub struct KeyboardState {
    pub buffer: Arc<Mutex<String>>,
    pub enabled: Arc<AtomicBool>,
    pub running: Arc<AtomicBool>,
}

impl KeyboardState {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(String::new())),
            enabled: Arc::new(AtomicBool::new(true)),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the current buffer content
    pub fn get_buffer(&self) -> String {
        self.buffer.lock().clone()
    }

    /// Clear the buffer
    pub fn clear_buffer(&self) {
        self.buffer.lock().clear();
    }

    /// Start listening for keyboard events in a background thread
    pub fn start_listening<F>(&self, on_trigger: F)
    where
        F: Fn(String) + Send + 'static,
    {
        if self.running.load(Ordering::Relaxed) {
            return; // Already running
        }

        let buffer = Arc::clone(&self.buffer);
        let enabled = Arc::clone(&self.enabled);
        let running = Arc::clone(&self.running);

        running.store(true, Ordering::Relaxed);

        thread::spawn(move || {
            let callback = move |event: Event| {
                if !enabled.load(Ordering::Relaxed) {
                    return;
                }

                match event.event_type {
                    EventType::KeyPress(key) => {
                        let mut buf = buffer.lock();

                        match key {
                            // Backspace — remove last char
                            Key::Backspace => {
                                buf.pop();
                            }
                            // Enter, Tab, Escape — combo breakers (reset buffer)
                            Key::Return | Key::Tab | Key::Escape => {
                                buf.clear();
                            }
                            // Space — check buffer then add space
                            Key::Space => {
                                let current = buf.clone();
                                buf.push(' ');
                                // Keep buffer manageable
                                if buf.len() > 200 {
                                    let excess = buf.len() - 200;
                                    buf.drain(..excess);
                                }
                                // Check if buffer ends with a known trigger
                                drop(buf);
                                on_trigger(current);
                            }
                            _ => {
                                // Try to convert key to character
                                if let Some(ch) = key_to_char(key) {
                                    buf.push(ch);
                                    // Keep buffer manageable
                                    if buf.len() > 200 {
                                        let excess = buf.len() - 200;
                                        buf.drain(..excess);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            };

            if let Err(e) = listen(callback) {
                eprintln!("Keyboard hook error: {:?}", e);
                running.store(false, Ordering::Relaxed);
            }
        });
    }

    pub fn set_enabled(&self, value: bool) {
        self.enabled.store(value, Ordering::Relaxed);
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

/// Convert rdev Key to character
fn key_to_char(key: Key) -> Option<char> {
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
        _ => None,
    }
}
