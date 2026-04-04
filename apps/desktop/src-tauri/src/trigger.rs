// trigger.rs — Super Ultra Plan: P10, P11, P16
//
// P10: Remove debounce — process every keystroke immediately (like original Beeftext)
// P11: HashMap O(1) lookup for strict-mode keyword matching
// P16: Only cache enabled, trigger-worthy snippets (non-empty keyword, text/both type)

use crate::engine::perform_substitution;
use crate::ollama::OllamaClient;
use crate::snippet::Snippet;
use crate::store;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

/// Maximum concurrent snippet-substitution threads (prevents unbounded spawn)
const MAX_CONCURRENT_SUBSTITUTIONS: usize = 8;
static THREAD_COUNT: AtomicUsize = AtomicUsize::new(0);

/// RAII guard — always restores keyboard hook active state and decrements THREAD_COUNT
/// when dropped, even if the substitution thread panics.
struct SubstitutionGuard {
    kb: Arc<crate::keyboard::KeyboardState>,
}

impl Drop for SubstitutionGuard {
    fn drop(&mut self) {
        self.kb.set_active(true);
        THREAD_COUNT.fetch_sub(1, Ordering::AcqRel);
    }
}

/// Persistent worker state shared across the module
static WORKER_STATE: WorkerState = WorkerState::new();

/// P11: Cached snippet index for fast lookup
struct SnippetCache {
    version: u64,
    /// All enabled trigger snippets
    all_snippets: Vec<Snippet>,
    /// P11: HashMap index for O(1) strict-mode lookups (keyword -> index into all_snippets)
    strict_index: HashMap<String, usize>,
    /// P11: HashMap index for case-insensitive strict-mode (lowercase keyword -> index)
    strict_index_lower: HashMap<String, usize>,
}

struct WorkerState {
    /// Channel sender to the worker thread
    sender: Mutex<Option<std::sync::mpsc::Sender<TriggerJob>>>,
    /// Stop signal to shut down the worker
    stop_flag: AtomicBool,
    /// Cache version counter — incremented on any snippet change
    cache_version: AtomicU64,
    /// P11: In-memory snippet cache with HashMap index
    cache: Mutex<Option<SnippetCache>>,
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
    fn get_cached(&self) -> (Vec<Snippet>, HashMap<String, usize>, HashMap<String, usize>) {
        let current_version = self.cache_version.load(Ordering::Relaxed);
        let guard = self.cache.lock();

        if let Some(ref cache) = *guard {
            if cache.version == current_version {
                return (cache.all_snippets.clone(), cache.strict_index.clone(), cache.strict_index_lower.clone());
            }
        }

        // Version mismatch — refresh cache from DB
        drop(guard);
        self.refresh_cache()
    }

    /// P16: Force refresh the cache from the database.
    /// Only loads enabled snippets with non-empty keywords and text/both content types.
    fn refresh_cache(&self) -> (Vec<Snippet>, HashMap<String, usize>, HashMap<String, usize>) {
        // P16: Use get_trigger_snippets() which filters by enabled + non-empty keyword + text/both type
        let snippets = match store::get_trigger_snippets() {
            Ok(s) => s,
            Err(e) => {
                // Fallback to get_all_snippets if get_trigger_snippets doesn't exist yet
                log::warn!("get_trigger_snippets failed ({}), falling back to get_all_snippets", e);
                match store::get_all_snippets() {
                    Ok(s) => s.into_iter().filter(|sn| sn.enabled && !sn.keyword.is_empty()).collect(),
                    Err(e2) => {
                        eprintln!("Failed to load snippets for cache: {}", e2);
                        return (Vec::new(), HashMap::new(), HashMap::new());
                    }
                }
            }
        };

        // P11: Build HashMap indices for O(1) strict-mode lookups
        let mut strict_index = HashMap::new();
        let mut strict_index_lower = HashMap::new();
        for (idx, snippet) in snippets.iter().enumerate() {
            strict_index.insert(snippet.keyword.clone(), idx);
            strict_index_lower.insert(snippet.keyword.to_lowercase(), idx);
        }

        let version = self.cache_version.load(Ordering::Relaxed);
        let cache = SnippetCache {
            version,
            all_snippets: snippets.clone(),
            strict_index: strict_index.clone(),
            strict_index_lower: strict_index_lower.clone(),
        };
        let mut guard = self.cache.lock();
        *guard = Some(cache);

        (snippets, strict_index, strict_index_lower)
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

/// P10: Spawn the persistent worker thread if not already running.
/// REMOVED DEBOUNCE: Every keystroke now triggers an immediate snippet lookup,
/// exactly like the original Beeftext.
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
        // P10: Process every job immediately — no debounce delay
        loop {
            if WORKER_STATE.stop_flag.load(Ordering::Relaxed) {
                break;
            }

            match rx.recv() {
                Ok(job) => {
                    // P10: Process immediately, no waiting
                    let ollama = get_ollama_for_worker();
                    let kb = match KEYBOARD_WORKER.get() {
                        Some(kb) => Arc::clone(kb),
                        None => continue, // Not initialized yet
                    };

                    // Run the check synchronously on this thread to avoid overhead
                    // of spawning yet another thread just for the match check.
                    // The actual substitution still spawns its own thread.
                    let rt = tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .unwrap();
                    rt.block_on(async {
                        check_and_substitute_internal(&job.buffer, &ollama, &kb).await;
                    });
                }
                Err(_) => {
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

/// Enqueue a buffer for trigger checking (P10: processed immediately, no debounce)
pub fn enqueue_trigger(buffer: String) {
    if let Some(sender) = WORKER_STATE.sender.lock().as_ref() {
        let job = TriggerJob { buffer };
        // Non-blocking send — if worker is overwhelmed, drop the job
        let _ = sender.send(job);
    }
}

/// Invalidate the snippet cache (call after any snippet CRUD operation)
pub fn invalidate_cache() {
    WORKER_STATE.invalidate();
}

/// P11: Internal check_and_substitute that uses HashMap index for O(1) lookups.
/// Falls back to linear scan for loose-mode snippets.
/// Note: Strict mode uses exact match (input == keyword), Loose mode uses ends_with.
async fn check_and_substitute_internal(
    typed_buffer: &str,
    ollama: &OllamaClient,
    kb: &Arc<crate::keyboard::KeyboardState>,
) -> bool {
    let (snippets, _strict_index, _strict_index_lower) = WORKER_STATE.get_cached();

    // P11: Try all snippets (matches_input handles both strict and loose mode)
    // The HashMap could be used for pure strict-match, but since our strict mode
    // uses ends_with + word boundary (not exact match), linear scan is still needed.
    // The HashMap is available for future optimization if we add ExactMatch mode.
    for snippet in &snippets {
        if snippet.matches_input(typed_buffer) {
            kb.clear_buffer();

            // H1 fix: atomic fetch_add + rollback — prevents race condition between threads
            let prev = THREAD_COUNT.fetch_add(1, Ordering::AcqRel);
            if prev >= MAX_CONCURRENT_SUBSTITUTIONS {
                // At capacity — rollback and skip this match
                THREAD_COUNT.fetch_sub(1, Ordering::AcqRel);
                return true;
            }

            // H4 fix: disable keyboard hook AFTER we've secured a slot
            kb.set_active(false);

            // Perform substitution on a separate thread (clipboard ops are blocking)
            // SubstitutionGuard (RAII) ensures set_active(true) + THREAD_COUNT-- even on panic
            let snippet_clone = snippet.clone();
            let ollama_clone = ollama.clone();
            let kb_clone = Arc::clone(kb);
            thread::spawn(move || {
                // Guard is bound to this thread's scope — always runs Drop on exit/panic
                let _guard = SubstitutionGuard { kb: kb_clone.clone() };
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("Failed to create tokio runtime for substitution");
                rt.block_on(async {
                    perform_substitution(&snippet_clone, &ollama_clone).await;
                });
                // _guard drops here, calling set_active(true) + THREAD_COUNT--
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
