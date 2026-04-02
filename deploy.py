import os
import sys
import json
import subprocess
import glob
from datetime import datetime, timezone

# Ensure the script is run from the project root
if not os.path.exists("package.json"):
    print("❌ Please run this script from the root of the BeeftextAI project.")
    sys.exit(1)

# Configuration & Paths
TAURI_DIR = os.path.join("apps", "desktop", "src-tauri")
TAURI_CONF_PATH = os.path.join(TAURI_DIR, "tauri.conf.json")
LATEST_JSON_PATH = "latest.json"

# The correct private key to ensure no mismatches with v0.4.5 clients
PRIVATE_KEY = "dW50cnVzdGVkIGNvbW1lbnQ6IHJzaWduIGVuY3J5cHRlZCBzZWNyZXQga2V5ClJXUlRZMEl5Y2N2L1Z2NmZ4ZGVVbU5WRFp2RWF0VWtHQStpeTBnMGhDTUF1ZStqbzY5TUFBQkFBQUFBQUFBQUFBQUlBQUFBQTlud2ZCQnZEVE1QUVowRDVYTlN0NHlDRFBNMUF3SUxmU053M2wrZ3FKQ1RvSUp6eVhLZ3VMSUE2SERpTTJ6aU5vWWtybldHc0RhOW9kVkhmbmdvN0NGTjVIQWRYQUxTUVpyMmN2bys0ZG9LNEtHeFMvbXYzN2QyT1VVL3QyOXBKSUZhUHFKMmZ1NUk5Cg=="
PRIVATE_KEY_PASSWORD = "790602"

def main():
    print("🚀 Starting BeeftextAI Automated Deployment...")
    
    # 1. Read the version
    if not os.path.exists(TAURI_CONF_PATH):
        print(f"❌ Could not find {TAURI_CONF_PATH}")
        sys.exit(1)
        
    with open(TAURI_CONF_PATH, 'r', encoding='utf-8') as f:
        tauri_conf = json.load(f)
        
    version = tauri_conf.get('version')
    if not version:
        print("❌ Could not find 'version' in tauri.conf.json")
        sys.exit(1)
        
    print(f"📦 Version to deploy: v{version}")
    
    # 2. Enter Release Notes
    notes = input(f"\n📝 Enter release notes for v{version} (or press Enter for default): ")
    if not notes.strip():
        notes = f"Release v{version}"

    # 3. Execute Builds
    # Inject signing keys safely via Environment Variables
    env = os.environ.copy()
    env["TAURI_SIGNING_PRIVATE_KEY"] = PRIVATE_KEY
    env["TAURI_SIGNING_PRIVATE_KEY_PASSWORD"] = PRIVATE_KEY_PASSWORD
    
    print("\n🔨 Building frontend assets (npm run build)...")
    subprocess.run(["npm", "run", "build"], shell=True, check=True, env=env)
    
    print("\n🔨 Building Tauri backend and signing installers (npm run tauri build)...")
    subprocess.run(["npm", "run", "tauri", "build"], shell=True, check=True, env=env)
    
    # 4. Process Artifacts
    nsis_dir = os.path.join(TAURI_DIR, "target", "release", "bundle", "nsis")
    msi_dir = os.path.join(TAURI_DIR, "target", "release", "bundle", "msi")
    
    exe_files = glob.glob(os.path.join(nsis_dir, "*.exe"))
    if not exe_files:
        print("❌ Error: Could not find generated .exe installer in nsis folder.")
        sys.exit(1)
    
    exe_file = exe_files[0]
    exe_name = os.path.basename(exe_file)
    sig_file = exe_file + ".sig"
    
    if not os.path.exists(sig_file):
        print(f"❌ Error: Signature file not found: {sig_file}. Did the build sign correctly?")
        sys.exit(1)
        
    with open(sig_file, 'r', encoding='utf-8') as f:
        signature = f.read().strip()
        
    print(f"\n🔑 Extracted Signature for {exe_name}")
    
    # 5. Sync latest.json Manifest
    print("📝 Syncing update manifest (latest.json)...")
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
        
    print("✅ Manifest synced!")
        
    # 6. Draft and Upload GitHub Release
    print(f"\n🚀 Creating GitHub Release v{version} and uploading assets...")
    
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
    
    print(f"\n🎉 Deployment successful! Beeftext AI v{version} is now live and the auto-updater is ready.")

if __name__ == "__main__":
    main()
