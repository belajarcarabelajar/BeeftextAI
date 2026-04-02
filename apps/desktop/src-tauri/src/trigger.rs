use crate::engine::perform_substitution;
use crate::ollama::OllamaClient;
use crate::snippet::Snippet;
use crate::store;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Debounce delay in milliseconds — wait this long after last keystroke before checking
const DEBOUNCE_DELAY_MS: u64 = 100;

/// Maximum concurrent snippet-substitution threads (prevents unbounded spawn)
const MAX_CONCURRENT_SUBSTITUTIONS: usize = 8;
static THREAD_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Persistent worker state shared across the module
static WORKER_STATE: WorkerState = WorkerState::new();

struct WorkerState {
    /// Channel sender to the worker thread
    sender: Mutex<Option<std::sync::mpsc::Sender<TriggerJob>>>,
    /// Stop signal to shut down the worker
    stop_flag: AtomicBool,
    /// Cache version counter — incremented on any snippet change
    cache_version: AtomicU64,
    /// In-memory snippet cache (keyword -> Snippet) for O(1) lookups
    cache: Mutex<Option<(u64, Vec<Snippet>)>>,
}

impl WorkerState {
    const fn new() -> Self {
        Self {
            sender: Mutex::new(None),
            stop_flag: AtomicBool::new(false),
            cache_version: AtomicU64::new(0),
            cache: Mutex::new(None),
        }
    }

    /// Get cached snippets, refreshing if version mismatch
    fn get_cached_snippets(&self) -> Vec<Snippet> {
        let current_version = self.cache_version.load(Ordering::Relaxed);
        let guard = self.cache.lock();

        if let Some((version, snippets)) = guard.as_ref() {
            if *version == current_version {
                return snippets.clone();
            }
        }

        // Version mismatch — refresh cache from DB
        drop(guard);
        self.refresh_cache()
    }

    /// Force refresh the cache from the database
    fn refresh_cache(&self) -> Vec<Snippet> {
        let snippets = match store::get_all_snippets() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to load snippets for cache: {}", e);
                return Vec::new();
            }
        };

        let version = self.cache_version.load(Ordering::Relaxed);
        let mut guard = self.cache.lock();
        *guard = Some((version, snippets.clone()));
        snippets
    }

    /// Invalidate the cache (call when snippets are modified)
    fn invalidate(&self) {
        self.cache_version.fetch_add(1, Ordering::Relaxed);
        let mut guard = self.cache.lock();
        *guard = None;
    }
}

/// A trigger job to be processed by the worker
struct TriggerJob {
    buffer: String,
}

/// Spawn the persistent worker thread if not already running
pub fn ensure_worker_running() {
    let mut sender_guard = WORKER_STATE.sender.lock();
    if sender_guard.is_some() {
        return;
    }

    let (tx, rx) = std::sync::mpsc::channel::<TriggerJob>();
    *sender_guard = Some(tx);
    drop(sender_guard);

    WORKER_STATE.stop_flag.store(false, Ordering::Relaxed);

    thread::spawn(move || {
        let mut last_trigger_time: Option<Instant> = None;
        let mut pending_buffer: Option<String> = None;

        loop {
            // If stop requested and no pending work, exit
            if WORKER_STATE.stop_flag.load(Ordering::Relaxed) && pending_buffer.is_none() {
                break;
            }

            // Calculate how long to wait for the next job
            let wait_duration = if let Some(last_time) = last_trigger_time {
                let elapsed = last_time.elapsed();
                if elapsed >= Duration::from_millis(DEBOUNCE_DELAY_MS) {
                    // Already past debounce window, don't wait
                    Duration::from_millis(0)
                } else {
                    Duration::from_millis(DEBOUNCE_DELAY_MS) - elapsed
                }
            } else {
                // Never triggered, use full debounce delay
                Duration::from_millis(DEBOUNCE_DELAY_MS)
            };

            // Wait for a job or timeout for debounce
            match rx.recv_timeout(wait_duration) {
                Ok(job) => {
                    // New buffer received — update pending and reset timer
                    pending_buffer = Some(job.buffer);
                    last_trigger_time = Some(Instant::now());
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    // Debounce window elapsed — process pending buffer if any
                    if let Some(buffer) = pending_buffer.take() {
                        last_trigger_time = None;
                        // Limit concurrent substitution threads — skip if at capacity
                        let current = THREAD_COUNT.load(Ordering::Relaxed);
                        if current >= MAX_CONCURRENT_SUBSTITUTIONS {
                            continue;
                        }
                        THREAD_COUNT.store(current + 1, Ordering::Relaxed);
                        let ollama = get_ollama_for_worker();
                        let kb = Arc::clone(KEYBOARD_WORKER.get().unwrap());
                        thread::spawn(move || {
                            let rt = tokio::runtime::Builder::new_current_thread()
                                .enable_all()
                                .build()
                                .unwrap();
                            rt.block_on(async {
                                check_and_substitute_internal(&buffer, &ollama, &kb).await;
                            });
                            THREAD_COUNT.fetch_sub(1, Ordering::Relaxed);
                        });
                    }
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    // Channel closed, exit worker
                    break;
                }
            }
        }

        // Clean up sender on exit
        let mut sender_guard = WORKER_STATE.sender.lock();
        *sender_guard = None;
    });
}

/// Get ollama client for worker thread
fn get_ollama_for_worker() -> OllamaClient {
    crate::get_ollama()
}

/// Keyboard state reference for worker thread
static KEYBOARD_WORKER: std::sync::OnceLock<Arc<crate::keyboard::KeyboardState>> = std::sync::OnceLock::new();

/// Set the keyboard state for the worker to use
pub fn set_keyboard_state(kb: Arc<crate::keyboard::KeyboardState>) {
    let _ = KEYBOARD_WORKER.set(kb);
}

/// Enqueue a buffer for trigger checking (debounced)
pub fn enqueue_trigger(buffer: String) {
    if let Some(sender) = WORKER_STATE.sender.lock().as_ref() {
        let job = TriggerJob {
            buffer,
        };
        // Non-blocking send — if worker is overwhelmed, drop the job
        let _ = sender.send(job);
    }
}

/// Invalidate the snippet cache (call after any snippet CRUD operation)
pub fn invalidate_cache() {
    WORKER_STATE.invalidate();
}

/// Internal check_and_substitute that uses the in-memory cache
async fn check_and_substitute_internal(
    typed_buffer: &str,
    ollama: &OllamaClient,
    kb: &Arc<crate::keyboard::KeyboardState>,
) -> bool {
    let snippets = WORKER_STATE.get_cached_snippets();

    for snippet in &snippets {
        if !snippet.enabled {
            continue;
        }

        if snippet.matches_input(typed_buffer) {
            kb.clear_buffer();
            kb.set_active(false); // Prevent simulated keystrokes from being captured

            // Limit concurrent substitution threads — skip if at capacity
            let current = THREAD_COUNT.load(Ordering::Relaxed);
            if current >= MAX_CONCURRENT_SUBSTITUTIONS {
                return true; // At capacity — skip this match
            }
            THREAD_COUNT.store(current + 1, Ordering::Relaxed);

            // Perform substitution on a separate thread (clipboard ops are blocking)
            let snippet_clone = snippet.clone();
            let ollama_clone = ollama.clone();
            let kb_clone = Arc::clone(kb);
            thread::spawn(move || {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                rt.block_on(async {
                    perform_substitution(&snippet_clone, &ollama_clone).await;
                });
                kb_clone.set_active(true); // Re-enable keyboard hook
                THREAD_COUNT.fetch_sub(1, Ordering::Relaxed);
            });

            return true;
        }
    }

    false
}

/// Stop the worker thread gracefully
#[allow(dead_code)]
pub fn stop_worker() {
    WORKER_STATE.stop_flag.store(true, Ordering::Relaxed);
    let mut sender_guard = WORKER_STATE.sender.lock();
    *sender_guard = None;
}
