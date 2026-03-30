use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use parking_lot::Mutex;
use rdev::{listen, Event, EventType, Key};

/// Shared state for the keyboard hook
pub struct KeyboardState {
    pub buffer: Arc<Mutex<String>>,
    pub enabled: Arc<AtomicBool>,
    pub running: Arc<AtomicBool>,
    pub shift_pressed: Arc<AtomicBool>,
}

impl KeyboardState {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(String::new())),
            enabled: Arc::new(AtomicBool::new(true)),
            running: Arc::new(AtomicBool::new(false)),
            shift_pressed: Arc::new(AtomicBool::new(false)),
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
        let shift_pressed = Arc::clone(&self.shift_pressed);

        running.store(true, Ordering::Relaxed);

        thread::spawn(move || {
            let callback = move |event: Event| {
                if !enabled.load(Ordering::Relaxed) {
                    return;
                }

                match event.event_type {
                    EventType::KeyPress(key) => {
                        match key {
                            Key::ShiftLeft | Key::ShiftRight => {
                                shift_pressed.store(true, Ordering::Relaxed);
                                return;
                            }
                            _ => {}
                        }

                        let mut buf = buffer.lock();
                        let is_shift = shift_pressed.load(Ordering::Relaxed);

                        match key {
                            // Backspace — remove last char
                            Key::Backspace => {
                                buf.pop();
                            }
                            // Enter, Tab, Escape — combo breakers (reset buffer)
                            Key::Return | Key::Tab | Key::Escape => {
                                buf.clear();
                            }
                            // Space or Punctuation — check buffer then add char
                            Key::Space | Key::Dot | Key::Comma | Key::SemiColon | Key::Slash | Key::BackSlash => {
                                let current = buf.clone();
                                if let Some(ch) = key_to_char(key, is_shift) {
                                    buf.push(ch);
                                }
                                
                                // Keep buffer manageable
                                if buf.len() > 200 {
                                    let excess = buf.len() - 200;
                                    buf.drain(..excess);
                                }
                                
                                // Trigger check
                                // We pass the buffer WITHOUT the trailing trigger char if we want exact matches,
                                // but the engine uses `matches_input` which might handle it.
                                // Let's pass the buffer as it was BEFORE this char for backward compatibility,
                                // OR better: the engine should check the buffer.
                                drop(buf);
                                on_trigger(current);
                            }
                            _ => {
                                // Try to convert key to character
                                if let Some(ch) = key_to_char(key, is_shift) {
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
                    EventType::KeyRelease(key) => {
                        match key {
                            Key::ShiftLeft | Key::ShiftRight => {
                                shift_pressed.store(false, Ordering::Relaxed);
                            }
                            _ => {}
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

/// Convert rdev Key to character, considering Shift state
fn key_to_char(key: Key, shift: bool) -> Option<char> {
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

