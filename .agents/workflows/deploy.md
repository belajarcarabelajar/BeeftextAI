---
description: Deploy a new version of BeefText AI with signed auto-updater
---

# Deploy BeefText AI Release

## Pre-flight
- Read the full deployment knowledge: `C:\Users\Tedi Rahmat\.gemini\antigravity\knowledge\tauri_v2_deployment_process\artifacts\tauri_v2_deploy.md`

## Steps

1. **Bump version** in all 3 files (must match):
   - `package.json`
   - `apps/desktop/src-tauri/Cargo.toml`
   - `apps/desktop/src-tauri/tauri.conf.json`

2. **Set signing environment variables** in the active PowerShell terminal:
// turbo
```powershell
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "790602"
$env:TAURI_SIGNING_PRIVATE_KEY = "dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5Y2N2L1Z2NmZ4ZGVVbU5WRFp2RWF0VWtHQStpeTBnMGhDTUF1ZStqbzY5TUFBQkFBQUFBQUFBQUFBQUlBQUFBQTlud2ZCQnZEVE1QUVowRDVYTlN0NHlDRFBNMUF3SUxmU053M2wrZ3FKQ1RvSUp6eVhLZ3VMSUE2SERpTTJ6aU5vWWtybldHc0RhOW9kVkhmbmdvN0NGTjVIQWRYQUxTUVpyMmN2bys0ZG9LNEtHeFMvbXYzN2QyT1VVL3QyOXBKSUZhUHFKMmZ1NUk9Cg=="
```

3. **Build frontend**:
// turbo
```powershell
npm run build
```

4. **Build Tauri backend with signing**:
```powershell
npm run tauri build
```
   - Wait for "Finished 2 updater signatures"
   - Verify `.exe.sig` (424 bytes) exists in `apps/desktop/src-tauri/target/release/bundle/nsis/`

5. **Read the signature** from the generated `.exe.sig` file

6. **Update `latest.json`** with the new version, signature, and download URL:
   - URL format: `https://github.com/belajarcarabelajar/BeeftextAI/releases/download/vX.Y.Z/BeefText.AI_X.Y.Z_x64-setup.exe`

7. **Create GitHub Release**:
```powershell
gh release delete vX.Y.Z --yes --cleanup-tag 2>$null
gh release create vX.Y.Z --title "BeefText AI vX.Y.Z" --notes "Release notes" "apps\desktop\src-tauri\target\release\bundle\nsis\BeefText.AI_X.Y.Z_x64-setup.exe" "apps\desktop\src-tauri\target\release\bundle\nsis\BeefText.AI_X.Y.Z_x64-setup.exe.sig" "latest.json"
gh release upload vX.Y.Z "apps\desktop\src-tauri\target\release\bundle\msi\BeefText.AI_X.Y.Z_x64_en-US.msi" "apps\desktop\src-tauri\target\release\bundle\msi\BeefText.AI_X.Y.Z_x64_en-US.msi.sig"
```

8. **Git commit & push** (exclude env files, secrets, .gitignore):
```powershell
git add package.json latest.json apps/desktop/src-tauri/Cargo.toml apps/desktop/src-tauri/Cargo.lock apps/desktop/src-tauri/tauri.conf.json apps/desktop/src-tauri/src/ src/ deploy.py
git commit -m "vX.Y.Z: description"
git push origin master
```

## Critical Warnings
- **NEVER** use `Get-Content` to read the private key — it injects CRLF and breaks signing
- **NEVER** change the pubkey in `tauri.conf.json` — breaks auto-updates for existing users
- **NEVER** commit `*.key*`, `secrets/`, `.env`, or `run_deploy.bat`
- The auto-updater URL must use `.exe` (not `.zip`) and match the exact GitHub asset name (dots, not spaces)
