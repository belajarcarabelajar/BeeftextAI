# Todo

## QA Report Fixes (2026-04-02) — All Complete

### ✅ Already Resolved (straight-line fixes)

| Issue | File | Fix |
|-------|------|-----|
| BLK-003 | `tauri.conf.json` | CSP enabled: `"csp": null` → restrictive policy |
| HIGH-006 | `capabilities/default.json` | Filesystem caps: `"path": "**"` → `$APP/**`, `$DOWNLOAD/**` |
| MED-006 | `src/i18n.ts:103` | `"SObrolan AI"` → `"Obrolan AI"` |
| MED-005 | `token.rs:33` | Stale comment `"max_tokens * 4 chars"` → `"max_tokens * 3 chars"` |
| MED-003 | `backup.rs:47` | Hardcoded `"0.1.0"` → `env!("CARGO_PKG_VERSION")` |
| MED-004 | `backup.rs:99` | Added `filename.contains("..")` guard for path traversal |
| MED-002 | `ollama.rs:121` | Added `resp.status().is_success()` check before `json()` in `chat()` |
| MED-001 | `variable.rs` | All 17 regex patterns → `once_cell::sync::Lazy<Regex>` (compiled once at startup) |
| HIGH-001 | `engine.rs:72,84` | Added `StrExt` trait with `char_count()` and `truncate_chars()` — UTF-8 safe |
| HIGH-003 | `trigger.rs:159` | `get_ollama_for_worker()` now calls `crate::get_ollama()` (respects user prefs) |

### ⚠️ Not Addressed (require user decisions / higher risk)

| Issue | Reason |
|-------|--------|
| BLK-001 | Private signing keys committed — requires key rotation, git history rewrite, CLAUDE.md update |
| BLK-002 | `#{powershell:path}` injection — requires per-snippet opt-in UI gate |
| HIGH-004 | Zero test coverage — no tests exist anywhere |
| HIGH-005 | No CI/CD pipeline — no `.github/workflows/` |

**Build status:** `cargo check` ✅ passes, `cargo build --release` ✅ builds, `npm run build` ✅ passes

---

## Session 2 — Remaining QA Fixes (2026-04-02)

### ✅ Already Resolved

| Issue | File | Fix |
|-------|------|-----|
| LOW-001 | `apps/desktop/src-tauri/log.txt` | Deleted + `git rm --cached` |
| LOW-002 | `src/App.tsx` | Removed production `console.log` in `sendMessage` |
| LOW-005 | `src/App.tsx` + `src/i18n.ts` | Both Indonesian confirm dialogs → `t("confirmKeywordDuplicate", ...)` with `{0}` placeholder |
| LOW-006 | `docs/Variables.md` | Clarified unsupported keys (modifier combos) vs supported (F1–F24, media keys) |
| LOW-004 | `trigger.rs` | Bounded thread pool: `AtomicUsize` counter limits to 8 concurrent threads, excess jobs dropped |
| HIGH-002 | `store.rs` | Replaced `std::sync::Mutex` → `parking_lot::Mutex`, removed all 22 `.unwrap()` calls |
| LOW-003 | `src/App.tsx` → `src/components/` | Extracted `SnippetEditor.tsx` (249L), `SettingsPanel.tsx` (275L); App.tsx 1694→1015L |

### ⚠️ Not Addressed (require user decisions / higher risk)

| Issue | Reason |
|-------|--------|
| BLK-001 | Private signing keys committed — requires key rotation, git history rewrite, CLAUDE.md update |
| BLK-002 | `#{powershell:path}` injection — requires per-snippet opt-in UI gate |
| HIGH-004 | Zero test coverage — no tests exist anywhere |
| HIGH-005 | No CI/CD pipeline — no `.github/workflows/` |

---

## Review Section

### What went well
- HIGH-002 (`parking_lot::Mutex`) was safe because `parking_lot` was already a direct dependency — no version risk
- LOW-004 bounded thread pool: used `AtomicUsize` counter + `try_acquire` pattern — no new dependencies, cross-platform compatible
- LOW-003 App.tsx split: `types.ts` and `utils.ts` shared modules prevented circular import issues

### Patterns to watch
- `std::sync::Semaphore` is Unix-only on Rust 1.94 — Windows MSVC build doesn't have it; use `AtomicUsize` counter pattern instead
- TypeScript `import("../types").BackupInfo` inline type syntax in `SettingsPanel.tsx` avoids creating a second named import — acceptable for internal components

### Next steps (blocked on user decisions)
- BLK-001: rotate signing keys and rewrite git history
- BLK-002: design per-snippet opt-in gate for `#{powershell:path}`
- HIGH-004/HIGH-005: CI/CD + test infrastructure
