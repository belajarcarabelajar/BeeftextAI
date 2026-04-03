import os
import sys
import json
import subprocess
import glob
from datetime import datetime, timezone

# Ensure the script is run from the project root
if not os.path.exists("package.json"):
    print("\u274c Please run this script from the root of the BeeftextAI project.")
    sys.exit(1)

# Configuration & Paths
TAURI_DIR = os.path.join("apps", "desktop", "src-tauri")
TAURI_CONF_PATH = os.path.join(TAURI_DIR, "tauri.conf.json")
LATEST_JSON_PATH = "latest.json"

# Signing keys — MUST be provided via environment variables
# Run before deploying:
#   export TAURI_SIGNING_PRIVATE_KEY="$(cat apps/desktop/src-tauri/main_new.key)"
#   export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="your_password"
PRIVATE_KEY = os.environ.get("TAURI_SIGNING_PRIVATE_KEY", "")
PRIVATE_KEY_PASSWORD = os.environ.get("TAURI_SIGNING_PRIVATE_KEY_PASSWORD", "")

def main():
    print("\ud83d\ude80 Starting BeeftextAI Automated Deployment...")

    if not PRIVATE_KEY:
        print("\u274c TAURI_SIGNING_PRIVATE_KEY environment variable is not set.")
        print("   Run: export TAURI_SIGNING_PRIVATE_KEY=\"$(cat apps/desktop/src-tauri/main_new.key)\"")
        print("        export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=\"your_password\"")
        sys.exit(1)

    # 1. Read the version
    if not os.path.exists(TAURI_CONF_PATH):
        print(f"\u274c Could not find {TAURI_CONF_PATH}")
        sys.exit(1)

    with open(TAURI_CONF_PATH, 'r', encoding='utf-8') as f:
        tauri_conf = json.load(f)

    version = tauri_conf.get('version')
    if not version:
        print("\u274c Could not find 'version' in tauri.conf.json")
        sys.exit(1)

    print(f"\ud83d\udce6 Version to deploy: v{version}")

    # 2. Enter Release Notes
    notes = input(f"\ud83d\udcdd Enter release notes for v{version} (or press Enter for default): ")
    if not notes.strip():
        notes = f"Release v{version}"

    # 3. Execute Builds
    # Inject signing keys safely via Environment Variables
    env = os.environ.copy()
    env["TAURI_SIGNING_PRIVATE_KEY"] = PRIVATE_KEY
    env["TAURI_SIGNING_PRIVATE_KEY_PASSWORD"] = PRIVATE_KEY_PASSWORD

    print("\n\ud83d\udd28 Building frontend assets (npm run build)...")
    subprocess.run(["npm", "run", "build"], shell=True, check=True, env=env)

    print("\n\ud83d\udd28 Building Tauri backend and signing installers (npm run tauri build)...")
    subprocess.run(["npm", "run", "tauri", "build"], shell=True, check=True, env=env)

    # 4. Process Artifacts
    nsis_dir = os.path.join(TAURI_DIR, "target", "release", "bundle", "nsis")
    msi_dir = os.path.join(TAURI_DIR, "target", "release", "bundle", "msi")

    exe_files = glob.glob(os.path.join(nsis_dir, "*.exe"))
    if not exe_files:
        print("\u274c Error: Could not find generated .exe installer in nsis folder.")
        sys.exit(1)

    exe_file = exe_files[0]
    exe_name = os.path.basename(exe_file)
    sig_file = exe_file + ".sig"

    if not os.path.exists(sig_file):
        print(f"\u274c Error: Signature file not found: {sig_file}. Did the build sign correctly?")
        sys.exit(1)

    with open(sig_file, 'r', encoding='utf-8') as f:
        signature = f.read().strip()

    print(f"\n\ud83d\udd11 Extracted Signature for {exe_name}")

    # 5. Sync latest.json Manifest
    print("\ud83d\udcdd Syncing update manifest (latest.json)...")
    with open(LATEST_JSON_PATH, 'r', encoding='utf-8') as f:
        latest = json.load(f)

    latest["version"] = version
    latest["notes"] = notes
    latest["pub_date"] = datetime.now(timezone.utc).strftime('%Y-%m-%dT%H:%M:%SZ')

    download_url = f"https://github.com/belajarcarabelajar/BeeftextAI/releases/download/v{version}/{exe_name}"

    latest["platforms"]["windows-x86_64"]["signature"] = signature
    latest["platforms"]["windows-x86_64"]["url"] = download_url

    with open(LATEST_JSON_PATH, 'w', encoding='utf-8') as f:
        json.dump(latest, f, indent=2)

    print("\u2705 Manifest synced!")

    # 6. Draft and Upload GitHub Release
    print(f"\n\ud83d\ude80 Creating GitHub Release v{version} and uploading assets...")

    # Delete if it accidentally exists
    subprocess.run(["gh", "release", "delete", f"v{version}", "--yes", "--cleanup-tag"], shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    msi_files = glob.glob(os.path.join(msi_dir, "*.msi"))

    gh_command = [
        "gh", "release", "create", f"v{version}",
        "--title", f"BeefText AI v{version}",
        "--notes", notes,
        exe_file, sig_file,
        LATEST_JSON_PATH
    ]

    if msi_files:
        msi_file = msi_files[0]
        gh_command.extend([msi_file, msi_file + ".sig"])

    subprocess.run(gh_command, shell=True, check=True)

    print(f"\n\ud83e\udd89 Deployment successful! Beeftext AI v{version} is now live and the auto-updater is ready.")

if __name__ == "__main__":
    main()
