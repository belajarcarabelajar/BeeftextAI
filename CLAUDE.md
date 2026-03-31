# Beeftext AI Deployment Guide

## 1. Preparation

Increase version in both `package.json` and `src-tauri/tauri.conf.json`.
The current latest release version is **0.1.6**.

## 2. Generate Release Build

Run the following commands:

```powershell
npm run build
npm run tauri build
```

This generates the installer and signatures in `src-tauri/target/release/bundle/`.

## 3. GitHub Release

Use the GitHub CLI (`gh`) to create a new release and upload the artifacts:

```powershell
gh release create v0.1.5 `
  --title "v0.1.5" `
  --notes "Release notes here..." `
  .\src-tauri\target\release\bundle\nsis\*.exe `
  .\src-tauri\target\release\bundle\nsis\*.exe.sig
```

## 4. Maintenance (Update latest.json)

After release:

1. Update `latest.json` in the project root with the new version and its date.
2. Provide the signature for each platform (from the `.sig` file generated during build).
3. Commit and push the changes to GitHub.

---

**Note:** The updater uses the public key configured in `src-tauri/tauri.conf.json`:
`dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEZFNzVBNzY2NkJBQjBFM0QKUldROURxdHJacWQxL25NQXV5MC94ZnF2Wmc5WGJoSWlNc0d4dDZXVTJ4RmpNUSt0QWRLVUNIOWcK`

The corresponding private key is stored in `src-tauri/main.key.raw` (password: `790602`).
