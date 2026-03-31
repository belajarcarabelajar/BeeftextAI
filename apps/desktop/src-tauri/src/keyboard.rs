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
            active: Arc::new(AtomicBool::new(true)),
        }
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
        let active = Arc::clone(&self.active);

        running.store(true, Ordering::Relaxed);

        thread::spawn(move || {
            let callback = move |event: Event| {
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
                            _ => {}
                        }

                        let mut buf = buffer.lock();
                        let is_shift = shift_pressed.load(Ordering::Relaxed);

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
                            // Space or Punctuation treated as normal keys now — no special delay/trailing behavior
                            Key::Space | Key::Dot | Key::Comma | Key::SemiColon | Key::Slash | Key::BackSlash => {
                                if let Some(ch) = key_to_char(key, is_shift) {
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
                            // Every normal key press — append to buffer and check for trigger immediately
                            _ => {
                                if let Some(ch) = key_to_char(key, is_shift) {
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

    /// Temporarily disable event processing (used during text substitution)
    pub fn set_active(&self, value: bool) {
        self.active.store(value, Ordering::Relaxed);
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

