// keyboard.rs — Super Ultra Plan: P1, P2, P3, P4
//
// P1: Native Win32 WH_KEYBOARD_LL hook via SetWindowsHookExW (replaces rdev::listen)
// P2: Dead key handling with wFlags=0x4 and dead-key state tracking
// P3: WH_MOUSE_LL hook clears buffer on click/wheel (combo breaker)
// P4: Full 256-byte keyboard state from GetKeyState for all modifier VKs
//
// rdev is kept as a fallback for non-Windows platforms.

use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::thread;
use parking_lot::Mutex;

#[cfg(target_os = "windows")]
use windows::Win32::UI::Input::KeyboardAndMouse::{
    ToUnicodeEx, GetKeyboardLayout, GetKeyState, HKL,
};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId,
    SetWindowsHookExW, UnhookWindowsHookEx, CallNextHookEx,
    GetMessageW, TranslateMessage, DispatchMessageW,
    WH_KEYBOARD_LL, WH_MOUSE_LL, KBDLLHOOKSTRUCT,
    MSG,
};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{WPARAM, LPARAM, LRESULT};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::GetCurrentThreadId;

/// Windows message constants
#[cfg(target_os = "windows")]
const WM_KEYDOWN: u32 = 0x0100;
#[cfg(target_os = "windows")]
const WM_SYSKEYDOWN: u32 = 0x0104;
#[cfg(target_os = "windows")]
const WM_LBUTTONDOWN: u32 = 0x0201;
#[cfg(target_os = "windows")]
const WM_RBUTTONDOWN: u32 = 0x0204;
#[cfg(target_os = "windows")]
const WM_MBUTTONDOWN: u32 = 0x0207;
#[cfg(target_os = "windows")]
const WM_MOUSEWHEEL: u32 = 0x020A;
/// Custom message to terminate the hook thread message loop
#[cfg(target_os = "windows")]
const WM_QUIT_HOOK: u32 = 0x0012; // WM_QUIT

/// VK codes for combo-breaker keys and modifiers
#[cfg(target_os = "windows")]
const VK_BACK: u32 = 0x08;
#[cfg(target_os = "windows")]
const VK_TAB: u32 = 0x09;
#[cfg(target_os = "windows")]
const VK_RETURN: u32 = 0x0D;
#[cfg(target_os = "windows")]
const VK_ESCAPE: u32 = 0x1B;
#[cfg(target_os = "windows")]
const VK_PRIOR: u32 = 0x21; // PageUp
#[cfg(target_os = "windows")]
const VK_NEXT: u32 = 0x22;  // PageDown
#[cfg(target_os = "windows")]
const VK_END: u32 = 0x23;
#[cfg(target_os = "windows")]
const VK_HOME: u32 = 0x24;
#[cfg(target_os = "windows")]
const VK_LEFT: u32 = 0x25;
#[cfg(target_os = "windows")]
const VK_UP: u32 = 0x26;
#[cfg(target_os = "windows")]
const VK_RIGHT: u32 = 0x27;
#[cfg(target_os = "windows")]
const VK_DOWN: u32 = 0x28;
#[cfg(target_os = "windows")]
const VK_INSERT: u32 = 0x2D;
#[cfg(target_os = "windows")]
const VK_DELETE: u32 = 0x2E;
#[cfg(target_os = "windows")]
const VK_LSHIFT: u32 = 0xA0;
#[cfg(target_os = "windows")]
const VK_RSHIFT: u32 = 0xA1;
#[cfg(target_os = "windows")]
const VK_CAPITAL: u32 = 0x14;
#[cfg(target_os = "windows")]
const VK_SHIFT: u32 = 0x10;

/// P4: All modifier VKs that need state tracking (mirrors original Beeftext)
#[cfg(target_os = "windows")]
const MODIFIER_VKS: &[u32] = &[
    0x10, // VK_SHIFT
    0xA0, // VK_LSHIFT
    0xA1, // VK_RSHIFT
    0x11, // VK_CONTROL
    0xA2, // VK_LCONTROL
    0xA3, // VK_RCONTROL
    0x12, // VK_MENU (Alt)
    0xA4, // VK_LMENU
    0xA5, // VK_RMENU
    0x5B, // VK_LWIN
    0x5C, // VK_RWIN
    0x14, // VK_CAPITAL (CapsLock)
];

/// Combo-breaker VK codes (navigation + delete/insert)
#[cfg(target_os = "windows")]
const BREAKER_VKS: &[u32] = &[
    VK_UP, VK_RIGHT, VK_DOWN, VK_LEFT,
    VK_PRIOR, VK_NEXT, VK_HOME, VK_END,
    VK_INSERT, VK_DELETE,
];

// ─── Global state for Win32 hook callbacks ──────────────────────────────────
// These must be global/static because Win32 hook callbacks are plain function pointers.
#[cfg(target_os = "windows")]
static GLOBAL_BUFFER: Mutex<Option<Arc<Mutex<String>>>> = Mutex::new(None);
#[cfg(target_os = "windows")]
static GLOBAL_ENABLED: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
#[cfg(target_os = "windows")]
static GLOBAL_ACTIVE: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);
#[cfg(target_os = "windows")]
static GLOBAL_CALLBACK: Mutex<Option<Arc<dyn Fn(String) + Send + Sync>>> = Mutex::new(None);
/// P14: Excluded application process names (lowercase)
#[cfg(target_os = "windows")]
static EXCLUDED_APPS: Mutex<Option<Arc<Mutex<Vec<String>>>>> = Mutex::new(None);

/// P2: Dead key state — stored as (vkCode, scanCode, keyboardState[256])
#[cfg(target_os = "windows")]
static DEAD_KEY: Mutex<Option<(u32, u32, [u8; 256])>> = Mutex::new(None);

/// Shared state for the keyboard hook
pub struct KeyboardState {
    pub buffer: Arc<Mutex<String>>,
    pub enabled: Arc<AtomicBool>,
    pub running: Arc<AtomicBool>,
    /// When false, skip processing events (used during text substitution to prevent feedback loop)
    active: Arc<AtomicBool>,
    /// P14: Excluded application names (lowercase)
    excluded_apps: Arc<Mutex<Vec<String>>>,
    /// Hook thread ID for posting WM_QUIT
    #[cfg(target_os = "windows")]
    hook_thread_id: Arc<Mutex<Option<u32>>>,
}

impl KeyboardState {
    pub fn new() -> Self {
        Self {
            buffer: Arc::new(Mutex::new(String::new())),
            enabled: Arc::new(AtomicBool::new(true)),
            running: Arc::new(AtomicBool::new(false)),
            active: Arc::new(AtomicBool::new(true)),
            excluded_apps: Arc::new(Mutex::new(Vec::new())),
            #[cfg(target_os = "windows")]
            hook_thread_id: Arc::new(Mutex::new(None)),
        }
    }

    /// Clear the buffer
    pub fn clear_buffer(&self) {
        self.buffer.lock().clear();
    }

    /// P14: Set excluded application list
    pub fn set_excluded_apps(&self, apps: Vec<String>) {
        let lowered: Vec<String> = apps.iter().map(|a| a.to_lowercase()).collect();
        *self.excluded_apps.lock() = lowered;
    }

    /// P14: Get the foreground process name (lowercase)
    #[cfg(target_os = "windows")]
    fn get_foreground_process_name() -> Option<String> {
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        use windows::Win32::System::ProcessStatus::GetModuleFileNameExW;

        unsafe {
            let hwnd = GetForegroundWindow();
            let mut pid: u32 = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 { return None; }

            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let mut buf = [0u16; 260];
            let len = GetModuleFileNameExW(Some(handle), None, &mut buf);
            let _ = windows::Win32::Foundation::CloseHandle(handle);
            if len == 0 { return None; }

            let path = String::from_utf16_lossy(&buf[..len as usize]);
            // Extract just the filename
            path.rsplit('\\').next().map(|s| s.to_lowercase())
        }
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

    /// P4: Build full 256-byte keyboard state array from GetKeyState()
    /// This mirrors the original Beeftext's approach of querying each modifier individually
    /// because GetKeyboardState() doesn't work correctly across processes.
    #[cfg(target_os = "windows")]
    fn build_keyboard_state() -> [u8; 256] {
        let mut state = [0u8; 256];
        unsafe {
            for &vk in MODIFIER_VKS {
                state[vk as usize] = (GetKeyState(vk as i32) & 0xFF) as u8;
            }
        }
        state
    }

    /// P1 + P2: Convert a keystroke to character(s) using ToUnicodeEx with dead key support.
    /// wFlags = 0x4 (bit 2 set) tells Win10+ to NOT consume the dead key state from kernel buffer.
    /// Returns the character(s) produced, or None for non-character keys.
    #[cfg(target_os = "windows")]
    fn process_key(vk: u32, scan_code: u32, keyboard_state: &[u8; 256]) -> Option<String> {
        unsafe {
            let hkl = Self::get_current_layout();
            let mut buf = [0u16; 10];

            // P2: wFlags = 0x4 (1<<2) — do NOT modify kernel keyboard state (Win10+)
            // This preserves dead keys so they compose properly with the next keystroke.
            let flags: u32 = 0x4;

            let result = ToUnicodeEx(
                vk,
                scan_code,
                keyboard_state,
                &mut buf,
                flags,
                Some(hkl),
            );

            if result == -1 {
                // P2: Dead key detected. The key has been stored in the kernel buffer.
                // With wFlags=0x4, the dead key is NOT consumed, so the next normal key
                // will automatically compose with it. We just need to track that we're
                // in a dead key state so we don't append garbage to the buffer.
                let mut dk = DEAD_KEY.lock();
                *dk = Some((vk, scan_code, *keyboard_state));
                return None;
            }

            if result > 0 {
                let s = String::from_utf16_lossy(&buf[..result as usize]);
                // P2: Clear dead key tracking since we got a normal result
                let mut dk = DEAD_KEY.lock();
                *dk = None;

                // Filter out control characters
                let filtered: String = s.chars().filter(|c| !c.is_control()).collect();
                if filtered.is_empty() {
                    return None;
                }
                return Some(filtered);
            }

            // result == 0: non-printable key (modifier, etc.)
            None
        }
    }

    /// P1: The native Win32 keyboard hook callback.
    /// Mirrors original Beeftext InputManager::keyboardProcedure exactly.
    #[cfg(target_os = "windows")]
    unsafe extern "system" fn keyboard_procedure(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        // Always call next hook if nCode < 0 (MSDN requirement)
        if n_code < 0 {
            return CallNextHookEx(None, n_code, w_param, l_param);
        }

        // Only process key-down events
        let msg = w_param.0 as u32;
        if msg != WM_KEYDOWN && msg != WM_SYSKEYDOWN {
            return CallNextHookEx(None, n_code, w_param, l_param);
        }

        // Check if enabled
        let enabled = {
            let guard = GLOBAL_ENABLED.lock();
            guard.as_ref().map(|e| e.load(Ordering::Relaxed)).unwrap_or(false)
        };
        if !enabled {
            return CallNextHookEx(None, n_code, w_param, l_param);
        }

        // Check if active (disabled during substitution to prevent feedback loop)
        let active = {
            let guard = GLOBAL_ACTIVE.lock();
            guard.as_ref().map(|a| a.load(Ordering::Relaxed)).unwrap_or(true)
        };
        if !active {
            return CallNextHookEx(None, n_code, w_param, l_param);
        }

        // P14: Check excluded applications
        let is_excluded = {
            let guard = EXCLUDED_APPS.lock();
            if let Some(ref apps_arc) = *guard {
                if let Some(proc_name) = Self::get_foreground_process_name() {
                    apps_arc.lock().iter().any(|app| proc_name == *app)
                } else {
                    false
                }
            } else {
                false
            }
        };
        if is_excluded {
            return CallNextHookEx(None, n_code, w_param, l_param);
        }

        // Extract keystroke data from KBDLLHOOKSTRUCT
        let kb_struct = &*(l_param.0 as *const KBDLLHOOKSTRUCT);
        let vk = kb_struct.vkCode;
        let scan_code = kb_struct.scanCode;

        // Ignore pure modifier keys (shift, capslock)
        if vk == VK_LSHIFT || vk == VK_RSHIFT || vk == VK_CAPITAL || vk == VK_SHIFT {
            return CallNextHookEx(None, n_code, w_param, l_param);
        }

        // Check for combo-breaker keys (navigation, tab, enter, escape)
        if BREAKER_VKS.contains(&vk) || vk == VK_TAB || vk == VK_RETURN || vk == VK_ESCAPE {
            // Clear buffer — combo breaker
            let guard = GLOBAL_BUFFER.lock();
            if let Some(ref buf) = *guard {
                buf.lock().clear();
            }
            return CallNextHookEx(None, n_code, w_param, l_param);
        }

        // Backspace — remove last char from buffer
        if vk == VK_BACK {
            let guard = GLOBAL_BUFFER.lock();
            if let Some(ref buf) = *guard {
                buf.lock().pop();
            }
            return CallNextHookEx(None, n_code, w_param, l_param);
        }

        // P4: Build full keyboard state array
        let keyboard_state = Self::build_keyboard_state();

        // P1 + P2: Process key through ToUnicodeEx
        if let Some(chars) = Self::process_key(vk, scan_code, &keyboard_state) {
            let guard = GLOBAL_BUFFER.lock();
            if let Some(ref buf_arc) = *guard {
                let mut buf = buf_arc.lock();
                for ch in chars.chars() {
                    buf.push(ch);
                }
                // Cap buffer at 200 chars
                if buf.len() > 200 {
                    let excess = buf.len() - 200;
                    buf.drain(..excess);
                }
                let current = buf.clone();
                drop(buf);
                drop(guard);

                // Fire the trigger callback
                let cb = GLOBAL_CALLBACK.lock();
                if let Some(ref callback) = *cb {
                    callback(current);
                }
            }
        }

        CallNextHookEx(None, n_code, w_param, l_param)
    }

    /// P3: The native Win32 mouse hook callback.
    /// Any click or mouse wheel event clears the buffer (combo breaker),
    /// exactly like the original Beeftext's mouseProcedure.
    #[cfg(target_os = "windows")]
    unsafe extern "system" fn mouse_procedure(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if n_code >= 0 {
            let msg = w_param.0 as u32;
            if msg == WM_LBUTTONDOWN || msg == WM_RBUTTONDOWN
                || msg == WM_MBUTTONDOWN || msg == WM_MOUSEWHEEL
            {
                // Check if enabled
                let enabled = {
                    let guard = GLOBAL_ENABLED.lock();
                    guard.as_ref().map(|e| e.load(Ordering::Relaxed)).unwrap_or(false)
                };
                if enabled {
                    let guard = GLOBAL_BUFFER.lock();
                    if let Some(ref buf) = *guard {
                        buf.lock().clear();
                    }
                }
            }
        }
        CallNextHookEx(None, n_code, w_param, l_param)
    }

    /// P1: Start listening using native Win32 hooks (primary) with rdev fallback.
    pub fn start_listening<F>(&self, on_trigger: F)
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        if self.running.load(Ordering::Relaxed) {
            return; // Already running
        }

        self.running.store(true, Ordering::Relaxed);

        // Set up global state for the Win32 hook callbacks
        #[cfg(target_os = "windows")]
        {
            *GLOBAL_BUFFER.lock() = Some(Arc::clone(&self.buffer));
            *GLOBAL_ENABLED.lock() = Some(Arc::clone(&self.enabled));
            *GLOBAL_ACTIVE.lock() = Some(Arc::clone(&self.active));
            *GLOBAL_CALLBACK.lock() = Some(Arc::new(on_trigger));
            *EXCLUDED_APPS.lock() = Some(Arc::clone(&self.excluded_apps));
        }

        #[cfg(target_os = "windows")]
        {
            let hook_thread_id = Arc::clone(&self.hook_thread_id);
            let running = Arc::clone(&self.running);

            thread::spawn(move || {
                unsafe {
                    // Store this thread's ID for WM_QUIT posting
                    let tid = GetCurrentThreadId();
                    *hook_thread_id.lock() = Some(tid);

                    // P1: Install keyboard hook
                    let kb_hook = SetWindowsHookExW(
                        WH_KEYBOARD_LL,
                        Some(Self::keyboard_procedure),
                        None,
                        0,
                    );

                    let kb_hook = match kb_hook {
                        Ok(h) => h,
                        Err(e) => {
                            log::error!("Failed to install keyboard hook: {:?}. Falling back to rdev.", e);
                            running.store(false, Ordering::Relaxed);
                            return;
                        }
                    };

                    // P3: Install mouse hook
                    let mouse_hook = SetWindowsHookExW(
                        WH_MOUSE_LL,
                        Some(Self::mouse_procedure),
                        None,
                        0,
                    );
                    let mouse_hook = match mouse_hook {
                        Ok(h) => Some(h),
                        Err(e) => {
                            log::warn!("Failed to install mouse hook: {:?}. Mouse clicks won't clear buffer.", e);
                            None
                        }
                    };

                    log::info!("Native Win32 hooks installed (keyboard + mouse)");

                    // P1: Message pump — required for low-level hooks to work.
                    // Without this, Windows will silently unregister our hooks after ~5 seconds.
                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }

                    // Cleanup on exit
                    let _ = UnhookWindowsHookEx(kb_hook);
                    if let Some(mh) = mouse_hook {
                        let _ = UnhookWindowsHookEx(mh);
                    }
                    running.store(false, Ordering::Relaxed);
                    log::info!("Native Win32 hooks uninstalled");
                }
            });
        }

        // Non-Windows fallback: use rdev
        #[cfg(not(target_os = "windows"))]
        {
            let buffer = Arc::clone(&self.buffer);
            let enabled = Arc::clone(&self.enabled);
            let running = Arc::clone(&self.running);
            let active = Arc::clone(&self.active);
            let on_trigger = Arc::new(on_trigger);

            thread::spawn(move || {
                use rdev::{listen, Event, EventType, Key};

                let callback = {
                    let buffer = Arc::clone(&buffer);
                    let enabled = Arc::clone(&enabled);
                    let active = Arc::clone(&active);
                    let on_trigger = Arc::clone(&on_trigger);
                    move |event: Event| {
                        if !enabled.load(Ordering::Relaxed) || !active.load(Ordering::Relaxed) {
                            return;
                        }
                        match event.event_type {
                            EventType::KeyPress(key) => {
                                let mut buf = buffer.lock();
                                match key {
                                    Key::Backspace => { buf.pop(); }
                                    Key::Return | Key::Tab | Key::Escape
                                    | Key::Home | Key::End
                                    | Key::UpArrow | Key::DownArrow
                                    | Key::LeftArrow | Key::RightArrow
                                    | Key::Insert | Key::Delete => { buf.clear(); }
                                    _ => {
                                        if let Some(ch) = event.name.as_ref().and_then(|n| n.chars().next()) {
                                            if !ch.is_control() {
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
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                };

                if let Err(e) = listen(callback) {
                    log::error!("rdev listen failed: {:?}", e);
                }
                running.store(false, Ordering::Relaxed);
            });
        }
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
