# 🔒 Security Audit — BeeftextAI Repository

> Audit date: 2026-04-04T23:38 WIB  
> Purpose: Pre-public visibility check for sensitive data leaks

## Summary

> [!CAUTION]
> **Critical secrets are present in both tracked files AND git history.** Making the repo public without cleanup **will expose your signing private key and password**.

---

## 🔴 Critical Findings (MUST fix before going public)

### 1. `deploy.ps1` — Signing private key + password in plaintext
- **Line 4:** `$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "790602"`
- **Line 5:** `$env:TAURI_SIGNING_PRIVATE_KEY = "dW50cnVzdGVkIGNv..."`
- **Status:** Currently tracked in git (`git ls-files` confirms)
- **Risk:** Anyone with these two values can sign fake updates that your installed users' apps will accept as genuine

### 2. `deploy.py` — Same password hardcoded
- **Line 31:** `PRIVATE_KEY_PASSWORD = "790602"`
- **Line 28-30:** Reads private key from `main_new.key` file path
- **Status:** Currently tracked in git

### 3. `.agents/workflows/deploy.md` — Full private key + password
- **Line 20-21:** Contains the complete base64 private key and password `790602`
- **Status:** Currently tracked in git

### 4. `temp/tmp.key` — A signing key file
- Contains raw key material (212 bytes)
- **Status:** Currently tracked in git (should be gitignored)

### 5. `docs/qa_report.md` — References password explicitly
- **Line 51:** Mentions password `790602` and describes where the private key is stored
- **Status:** Currently tracked in git

### 6. `CLAUDE.md` — References to key storage locations
- **Line 17:** Mentions `new_keys.txt` as private key source
- **Line 66:** Contains old public key (this is safe — pubkey is not secret)
- **Line 68:** References `main.key.raw` as private key location
- **Risk:** Low (doesn't contain actual secrets), but reveals key storage patterns

---

## 🟡 Git History Findings (MUST scrub before going public)

The following sensitive files were committed to git history at some point. Even though they may be gitignored now, **the data is still in git history**:

| Commit | Files |
|--------|-------|
| `dd82056` | `src-tauri/main.key`, `src-tauri/main.key.pub` |
| `cfcef97` | `src-tauri/main.key.raw` |
| `57ff5a1` | `clean.key`, `secret.key`, `tmp.key` |
| `2f297fa` | `apps/desktop/src-tauri/main.key`, `main.key.pub`, `main.key.raw` |
| `b47f7d9` | `apps/desktop/src-tauri/private_key.pem` |
| `7fa2d2a` | `deploy.ps1` (with embedded private key + password) |

---

## ✅ Clean Files (no issues)

- `src/` — All frontend TypeScript/TSX files: **clean**
- `apps/desktop/src-tauri/src/` — All Rust source files: **clean**
- `tauri.conf.json` — Contains only the **public** key (safe)
- `latest.json` — Contains only signatures and download URLs (safe)
- `package.json`, `Cargo.toml` — **clean**
- `index.html`, `index.css` — **clean**

---

## 🛠 Required Cleanup Steps

### Step 1: Remove sensitive tracked files from git

```
git rm --cached deploy.ps1
git rm --cached temp/tmp.key
```

### Step 2: Sanitize files that should stay but need secrets removed

- `deploy.py` — Replace hardcoded password with `os.environ.get()` (already partially done)
- `.agents/workflows/deploy.md` — Replace inline key/password with placeholder
- `docs/qa_report.md` — Redact the password reference on line 51

### Step 3: Update `.gitignore`

```
deploy.ps1
run_deploy.bat
temp/
```

### Step 4: Scrub git history with `git filter-repo`

```powershell
pip install git-filter-repo
git filter-repo --invert-paths --path deploy.ps1 --path temp/tmp.key --path clean.key --path secret.key --path tmp.key --path src-tauri/main.key --path src-tauri/main.key.pub --path src-tauri/main.key.raw --path apps/desktop/src-tauri/main.key --path apps/desktop/src-tauri/main.key.pub --path apps/desktop/src-tauri/main.key.raw --path apps/desktop/src-tauri/private_key.pem --force
```

### Step 5: Force push the cleaned history

```powershell
git remote add origin https://github.com/belajarcarabelajar/BeeftextAI.git
git push origin master --force
```

### Step 6: Rotate keys (RECOMMENDED)

Since the private key + password have been in git, ideally you should:
1. Generate a new signing keypair with `npx tauri signer generate`
2. Update `tauri.conf.json` with the new pubkey
3. Use the new private key for all future builds

However, this **will break auto-updates for existing users** — they'd need to manually install the new version. If you only have a small number of users (or just yourself), this is fine.
