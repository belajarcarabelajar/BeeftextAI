# 🛡️ QA Report — BeefText AI v0.4.1

**Generated:** 2026-04-02  
**Repository:** `belajarcarabelajar/BeeftextAI`  
**Audit Scope:** Full repository — all Rust source files, React frontend, configs, dependencies, build system

---

## 📋 Executive Summary

BeefText AI is a capable, well-conceived desktop text-expansion utility with an impressively rich feature set for a solo-developed project. The Rust backend is modular and the core snippet-matching and clipboard-injection pipeline is logically sound. However, the repository has **two critical security vulnerabilities that must be resolved before public release**: private signing keys committed to the git history, and a command injection vector via the `#{powershell:path}` template variable. Additionally, the application ships with zero automated tests, no CI/CD pipeline, and a Content Security Policy that is fully disabled — a significant operational risk posture for any public release. With those blockers addressed, the application is otherwise functionally reasonable for a v1 release.

---

## 🚦 Deployment Readiness Verdict

> ## ⚠️ CONDITIONAL
>
> The application **must not be deployed publicly** until the three blocking issues below are resolved. The private signing key committed to this repository is a supply-chain security failure — any actor with repository access can forge signed auto-updates. This is not a theoretical risk; it is an active, exploitable vulnerability.
>
> Once the three blockers are fixed, the application is deployable as a personal/internal tool with the medium-priority issues tracked for a follow-up release.

---

## 📊 Finding Summary

| Severity | Count |
|----------|-------|
| 🔴 Critical | 3 |
| 🟠 High | 6 |
| 🟡 Medium | 7 |
| 🔵 Low | 6 |
| ⚪ Info | 4 |

---

## 🔴 Blocking Issues

### BLK-001 · Private Signing Keys Committed to Git

**File:** `apps/desktop/src-tauri/main.key`, `main.key.pub`, `main.key.raw`  
**Severity:** 🔴 Critical  
**Category:** Secrets & Data Exposure (CWE-312)

**Evidence:** Running `git ls-files` confirms all three key files are tracked:
```
apps/desktop/src-tauri/main.key
apps/desktop/src-tauri/main.key.pub
apps/desktop/src-tauri/main.key.raw
```
These files contain the Tauri NSIS auto-updater signing private key (password `YOUR_PASSWORD`, documented in `CLAUDE.md:68`). Additionally, `CLAUDE.md:21` contains the entire base64-encoded private key inline as a copy-paste snippet.

**Impact:** Any person with read access to this repository can extract the private key and sign a malicious binary that the auto-updater will accept as legitimate. This is a complete compromise of the update integrity guarantee.

**Remediation:**
1. Rotate the signing keypair immediately: `npm run tauri signer generate -w newkey.key`
2. Update `tauri.conf.json` with the new public key.
3. Remove the key files from git history: `git filter-repo --path apps/desktop/src-tauri/main.key --invert-paths` (and the other two files)
4. Remove the plain-text key and password from `CLAUDE.md`.
5. Add `apps/desktop/src-tauri/*.key` and `apps/desktop/src-tauri/*.key.*` to `.gitignore`.
6. Store the private key **only** as an environment variable (`TAURI_SIGNING_PRIVATE_KEY`) in a secure secrets manager (GitHub Actions Secrets, 1Password CLI, etc.).

---

### BLK-002 · Command Injection via `#{powershell:path}` Template Variable

**File:** `apps/desktop/src-tauri/src/variable.rs:144–189`  
**Severity:** 🔴 Critical  
**Category:** Command Injection (CWE-78 / OWASP A03)

**Evidence:** The `#{powershell:path}` variable executes an arbitrary PowerShell script located at a user-supplied `path`. The path is passed directly to the shell:
```rust
let ps = format!("& '{}'", path);
Command::new("powershell")
    .args(["-WindowStyle", "Hidden", "-Command", &ps])
```
A snippet containing `#{powershell:C:\Temp\evil.ps1}` — or imported from a crafted JSON backup — executes that script with the user's full privileges when the snippet keyword is triggered. There is no path validation, allowlisting, or sandboxing.

> [!NOTE]
> This feature is an intentional port from the original Beeftext application and is prominently documented in `docs/Variables.md`. It is a power-user feature by design. However, it should be protected by a clear user-consent mechanism.

**Remediation:**
1. **Short-term:** Add a per-snippet "Allow PowerShell execution" checkbox that defaults to `false`. Require explicit opt-in.
2. **Short-term:** Validate that `path` is an absolute path to an existing `.ps1` file, rejecting anything with injection characters.
3. **Long-term:** Warn users prominently when importing a backup that contains `#{powershell:...}` variables.

---

### BLK-003 · Content Security Policy Fully Disabled

**File:** `apps/desktop/src-tauri/tauri.conf.json:25`  
**Severity:** 🔴 Critical  
**Category:** Security Misconfiguration (CWE-16)

**Evidence:**
```json
"security": {
  "csp": null
}
```
A `null` CSP allows the WebView to load arbitrary scripts, styles, and resources from any origin. Combined with the permissive `fs:allow-write-text-file` and `fs:allow-read-file` capabilities (which grant access to `"path": "**"` — the entire filesystem), a XSS vulnerability in the frontend could escalate to arbitrary file read/write.

**Remediation:**
```json
"security": {
  "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; font-src 'self' https://fonts.gstatic.com; img-src 'self' data: blob:; connect-src 'self' http://localhost:11434"
}
```

---

## 🟠 High Priority Issues

### HIGH-001 · UTF-8 Panic in String Byte Slicing

**File:** `apps/desktop/src-tauri/src/engine.rs:72,84`  
**Severity:** 🟠 High  
**Category:** Correctness (CWE-131)

**Evidence:**
```rust
// Line 72
let preview = if expanded.len() > 50 { format!("{}...", &expanded[..50]) } else { expanded.clone() };
// Line 84
ContentType::Text => if expanded.len() > 80 { format!("{}...", &expanded[..80]) } else { expanded },
```
`String::len()` returns **byte** length, not character count. Slicing at byte 50 or 80 on a string containing multi-byte UTF-8 characters (e.g., Arabic, Chinese, emoji) will **panic at runtime** with `byte index is not a char boundary`.

**Remediation:**
```rust
// Use char_indices to find a safe boundary
fn safe_truncate(s: &str, max_chars: usize) -> &str {
    s.char_indices().nth(max_chars).map_or(s, |(i, _)| &s[..i])
}
```

---

### HIGH-002 · Mutex Poison Risk — Database Layer

**File:** `apps/desktop/src-tauri/src/store.rs` (all 22 `DB.lock().unwrap()` calls)  
**Severity:** 🟠 High  
**Category:** Reliability (CWE-667)

**Evidence:** The global database connection is protected by `std::sync::Mutex`. If any thread panics while holding this lock, the mutex becomes "poisoned" and every subsequent `unwrap()` call on `DB.lock()` will also panic, crashing the entire application. This affects all 22 database access functions.

**Remediation:** Either use `parking_lot::Mutex` (already used in `keyboard.rs` and `trigger.rs` — it never poisons) for consistency, or handle the `PoisonError`:
```rust
let guard = DB.lock().unwrap_or_else(|p| p.into_inner());
```

---

### HIGH-003 · Hardcoded Ollama Model in Worker Thread (Ignores User Preferences)

**File:** `apps/desktop/src-tauri/src/trigger.rs:159–165`  
**Severity:** 🟠 High  
**Category:** Correctness

**Evidence:**
```rust
fn get_ollama_for_worker() -> OllamaClient {
    OllamaClient::new(
        "http://localhost:11434".to_string(),
        "nemotron-3.5-super:8b".to_string(),   // ← hardcoded, different from default!
        "nomic-embed-text".to_string(),
    )
}
```
The trigger worker uses `nemotron-3.5-super:8b` — a model name that doesn't match the settings-configured default (`nemotron-3-super:cloud`). Users who configure a custom model in Settings will find that `#{ai:prompt}` template variables in triggered snippets silently use the wrong model.

**Remediation:** Replace with `crate::get_ollama()` (already defined in `lib.rs`) or read preferences via `store::get_preference`.

---

### HIGH-004 · Zero Automated Test Coverage

**File:** Entire repository  
**Severity:** 🟠 High  
**Category:** Test Coverage

No test files exist — not a single unit test, integration test, or property-based test. The critical matching logic in `snippet.rs`, the variable evaluation in `variable.rs`, and the token estimation in `token.rs` are all entirely untested. A regression in the snippet matching algorithm (the core feature) would go undetected until a user reports it.

**Remediation (prioritized):**
1. Add unit tests to `snippet.rs::matches_input` — strict mode, loose mode, word boundary, case sensitivity
2. Add unit tests to `token.rs::estimate_tokens` and `truncate_to_tokens`
3. Add unit tests to `variable.rs` for each variable type with mock Ollama client
4. Add unit tests to `migration.rs::import_beeftext_json` with sample fixture files

---

### HIGH-005 · No CI/CD Pipeline

**File:** Repository root (no `.github/workflows/`)  
**Severity:** 🟠 High  
**Category:** Operational Readiness

There is no automated build, test, or lint pipeline. Code is pushed directly to master and released manually. A CI pipeline running `cargo check`, `cargo clippy`, and `npm run build` on every push would have caught the stale comment, dead code, and other issues found in this audit.

**Remediation:** Add a minimal GitHub Actions workflow:
```yaml
# .github/workflows/ci.yml
on: [push, pull_request]
jobs:
  check:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: npm ci
      - run: npm run build
      - run: cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
      - run: cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml -- -D warnings
```

---

### HIGH-006 · Overly Permissive Filesystem Capabilities

**File:** `apps/desktop/src-tauri/capabilities/default.json:12–19`  
**Severity:** 🟠 High  
**Category:** Least Privilege (CWE-250)

**Evidence:**
```json
{ "identifier": "fs:allow-write-text-file", "allow": [{ "path": "**" }] },
{ "identifier": "fs:allow-read-file",       "allow": [{ "path": "**" }] }
```
The wildcard `"path": "**"` grants the WebView unrestricted read and write access to the entire filesystem. Any JavaScript running in the WebView (including via future XSS) can read `C:\Users\...\AppData\Roaming` files, environment files, or write anywhere.

**Remediation:** Restrict to only the paths the app actually needs:
```json
{ "identifier": "fs:allow-write-text-file", "allow": [{ "path": "$APP/**" }, { "path": "$DOWNLOAD/**" }] },
{ "identifier": "fs:allow-read-file",       "allow": [{ "path": "$APP/**" }, { "path": "$DOWNLOAD/**" }] }
```

---

## 🟡 Medium & Low Priority Issues

### Medium Issues

**MED-001 · Regex Objects Compiled on Every Invocation**  
`variable.rs:222–431` — All 17 regex patterns (e.g., `Regex::new(r"#{ai:...")`) are compiled inside `evaluate_variables()`, which is called on every snippet trigger. Regex compilation is expensive. Use `once_cell::sync::Lazy` to compile them once at startup. Estimated 10–50ms savings per expansion.

**MED-002 · API Response Not Checked Before JSON Parsing (chat endpoint)**  
`ollama.rs:121–123` — The `/api/chat` endpoint calls `resp.json()` without first checking `resp.status().is_success()`. If Ollama returns a 4xx/5xx, the error message is lost and replaced with an unhelpful JSON parse error. The `embed()` method correctly checks status (line 136) but `chat()` does not.

**MED-003 · `backup.rs` Hardcodes App Version as `"0.1.0"`**  
`backup.rs:47` — `app_version: "0.1.0".to_string()` — This has been stale since v0.2.0. Backups report the wrong app version in their metadata.  
Fix: Use `env!("CARGO_PKG_VERSION")` at compile time.

**MED-004 · Path Traversal Risk in Backup Restore**  
`backup.rs:100` — `backup_dir().join(filename)` — The `filename` parameter is user-supplied from the frontend with no sanitization. A filename like `../../../Windows/System32/notepad.exe` could point to an arbitrary location.  
Fix: `assert!(!filename.contains(".."))` or use `Path::file_name()` to strip directory components.

**MED-005 · Stale Comment in `token.rs`**  
`token.rs:33` — Comment says `"Take roughly max_tokens * 4 chars"` but the code computes `max_tokens * CHARS_PER_TOKEN` where `CHARS_PER_TOKEN = 3`. The comment is incorrect.

**MED-006 · i18n Typo — Indonesian Translation**  
`i18n.ts:103` — `"SObrolan AI"` should be `"Obrolan AI"`. The capital `O` was split by an accidental `S` prefix.

**MED-007 · `docs/` Directory Not Tracked by Git**  
`git status` shows `?? docs/` — the `docs/` directory (containing `QA_Agent_System_Instructions.md` and `Variables.md`) is not tracked by git. If this documentation is intended for developers and contributors, it should be committed. If intentionally private, no action needed.

### Low Issues

**LOW-001 · 8 Compiler Warnings Suppressed (from committed `log.txt`)**  
`log.txt` captures a build from v0.1.0 with 8 warnings (`unused import`, `dead_code`, `unused field`). While most of these are now resolved in the current source, the log.txt file itself should be removed from git — it's a development artifact.  
Fix: `git rm apps/desktop/src-tauri/log.txt` and add `*.txt` to the Tauri `.gitignore`.

**LOW-002 · Frontend Has One `console.log` Debug Statement**  
`App.tsx:1066` — `console.log(\`[TOKEN] sendMessage | ...\`)` — This debug logging statement leaks token count metadata to the browser console in production builds. Not a security risk but unprofessional in release builds.  
Fix: Wrap in `if (import.meta.env.DEV)` or remove.

**LOW-003 · Monolithic `App.tsx` (1694 lines, 83KB)**  
`src/App.tsx` — The entire React application — layout, all pages, all modals, all utility functions — lives in a single file. This makes it extremely difficult to navigate, test individually, or reuse components. No functional bug, but a significant maintainability concern.

**LOW-004 · `trigger.rs` Spawns Unbounded Threads on Match**  
`trigger.rs:212` — Each matched snippet spawns a new `thread` + `tokio::runtime`. Under adversarial conditions (very short keyword, rapid typing), this could create a burst of threads before the `kb.set_active(false)` guard takes effect.

**LOW-005 · Hard-coded Confirmation Dialogs in Indonesian**  
`App.tsx:564,1194` — `window.confirm("Apakah kamu yakin?...")` — Despite the app supporting English, Indonesian, and bilingual modes, these two confirmation dialogs are always in Indonesian. They should use the i18n system.

**LOW-006 · `docs/Variables.md` describes keys not supported in BeefText AI**  
`docs/Variables.md:196–203` — The documentation lists media keys (`volumeMute`, `volumeUp`, `mediaNextTrack`, etc.) and `f13-f24` as supported by `#{key:}`. The implementation in `variable.rs:key_name_to_rdev()` does not map any of these keys (lines 23–99). Users who rely on the documentation will be confused.

---

## ✅ Operational Readiness Checklist

| Item | Status | Notes |
|------|--------|-------|
| Build process completes without errors | ✅ PASS | `npm run build` succeeds cleanly in 1.02s |
| TypeScript compilation clean | ✅ PASS | `tsc -p config/runtime/tsconfig.json` passes |
| Lockfiles committed (`package-lock.json`, `Cargo.lock`) | ✅ PASS | Both present and committed |
| Build artifacts excluded from VCS (`dist/`, `target/`) | ✅ PASS | Properly `.gitignore`'d |
| No known CVEs in npm dependencies | ✅ PASS | `npm audit`: 0 vulnerabilities across 136 packages |
| Sensitive files excluded from VCS | ❌ FAIL | `main.key*`, `log.txt` committed (see BLK-001, LOW-001) |
| CI/CD pipeline exists | ❌ FAIL | No `.github/workflows/` directory |
| Tests run in CI | ❌ FAIL | No tests exist |
| Content Security Policy configured | ❌ FAIL | `"csp": null` (see BLK-003) |
| Filesystem permissions follow least privilege | ❌ FAIL | `"path": "**"` wildcard (see HIGH-006) |
| Private keys managed via secrets manager | ❌ FAIL | Keys committed to git (see BLK-001) |
| Containerization / Infrastructure | N/A | Desktop application |
| Health check / monitoring endpoint | N/A | Desktop application |
| Database migrations versioned | PARTIAL | Schema migration via `pragma_table_info`, no version tracking |
| Auto-updater configured and functional | ✅ PASS | `latest.json` present, NSIS updater configured |

---

## 📈 Test Coverage Summary

**Coverage:** 0% — No test files exist anywhere in the repository.

**Critical untested paths (ranked by risk):**

| Rank | Path | Risk |
|------|------|------|
| 1 | `snippet.rs::matches_input` | Core feature — any regression silently breaks all expansions |
| 2 | `variable.rs::evaluate_variables` | 17 variable types, all untested |
| 3 | `engine.rs::perform_substitution` | UTF-8 panic on line 72, 84 undetected |
| 4 | `token.rs::truncate_to_tokens` | Token budget correctness — affects AI chat quality |
| 5 | `store.rs::init_db` | Schema migration correctness — data loss on upgrade |
| 6 | `migration.rs::import_beeftext_json` | Import parser edge cases |
| 7 | `backup.rs::restore_backup` | Data restoration integrity |
| 8 | `keyboard.rs::vk_to_char_layout_aware` | International layout correctness |

**Test quality assessment:** N/A — no tests to assess.

---

## 🔒 Security Summary

**Overall Security Posture: WEAK**

| Category | Assessment |
|---|---|
| Secrets management | ❌ Private key committed to git |
| Input validation | ⚠️ `#{powershell:path}` accepts any path without validation |
| Injection vulnerabilities | 🔴 Command injection via powershell variable |
| Content Security Policy | ❌ Disabled entirely |
| Filesystem permissions | ❌ Unrestricted wildcard access |
| Dependency vulnerabilities | ✅ Zero known CVEs (npm audit clean) |
| Data privacy | ✅ Fully on-device, no cloud calls |
| Authentication | N/A (local desktop app) |

The data privacy model is genuinely excellent — everything is local, no telemetry, no cloud. The security failures are all configuration and implementation issues that can be fixed without architectural changes.

---

## 📝 Recommendations (Ordered by Priority)

### Immediate (Before Any Public Release)

1. **Rotate the signing keys** (BLK-001) — Revoke the currently committed key, generate a new keypair, remove the old keys from git history, store only in environment secrets. **This is the most urgent action.**

2. **Enable CSP** (BLK-003) — Set a restrictive Content Security Policy in `tauri.conf.json`. This is a one-line change.

3. **Restrict filesystem capabilities** (HIGH-006) — Replace the `"path": "**"` wildcard with specific allowed directories.

### Before Next Feature Release

4. **Fix UTF-8 byte slice panic** (HIGH-001) — Replace `&expanded[..50]` and `&expanded[..80]` in `engine.rs` with character-safe truncation. This is a latent crash waiting for a non-ASCII user.

5. **Fix hardcoded Ollama model in trigger worker** (HIGH-003) — Replace `get_ollama_for_worker()` with `crate::get_ollama()` so user model preference is respected.

6. **Replace `std::sync::Mutex` with `parking_lot::Mutex` in `store.rs`** (HIGH-002) — Eliminate the poison risk across all 22 database operations.

7. **Add a user-consent gate for `#{powershell:path}`** (BLK-002) — Require explicit per-snippet opt-in and display a warning when importing backups with powershell variables.

### Technical Debt (Ongoing)

8. **Compile regexes statically** (MED-001) — Use `once_cell::sync::Lazy<Regex>` for all 17 patterns in `variable.rs`.

9. **Add unit tests for `snippet.rs`, `token.rs`, `variable.rs`** (HIGH-004) — Start with the three modules that have the most logic and the most risk.

10. **Add a minimal GitHub Actions CI workflow** (HIGH-005) — Even just `cargo check` and `npm run build` on every push catches regressions early.

11. **Fix API response check in `ollama.rs::chat()`** (MED-002), hardcoded version in `backup.rs` (MED-003), i18n typo (MED-006), and stale code comment in `token.rs` (MED-005).

12. **Decompose `App.tsx`** (LOW-003) — Extract pages and modals into separate component files as the codebase grows.

---

*Report compiled by: AI QA Agent per `docs/QA_Agent_System_Instructions.md`*  
*Repository state: commit `c2423fc` (HEAD → master)*
