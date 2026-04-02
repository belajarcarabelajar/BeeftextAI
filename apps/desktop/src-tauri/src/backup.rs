use std::path::PathBuf;
use std::fs;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use crate::store;
use crate::snippet::Snippet;
use crate::group::Group;

#[derive(Serialize, Deserialize)]
pub struct BackupData {
    pub version: String,
    pub created_at: String,
    pub app_version: String,
    pub snippets: Vec<Snippet>,
    pub groups: Vec<Group>,
    pub preferences: Vec<(String, String)>,
}

#[derive(Serialize)]
pub struct BackupInfo {
    pub filename: String,
    pub created_at: String,
    pub snippet_count: usize,
    pub group_count: usize,
    pub size_bytes: u64,
}

/// Get the backup directory path
fn backup_dir() -> PathBuf {
    let dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("BeeftextAI")
        .join("backups");
    fs::create_dir_all(&dir).ok();
    dir
}

/// Create a backup of all data
pub fn create_backup() -> Result<BackupInfo, String> {
    let snippets = store::get_all_snippets()?;
    let groups = store::get_all_groups()?;
    let prefs = store::get_all_preferences()?;

    let data = BackupData {
        version: "1.0".to_string(),
        created_at: Utc::now().to_rfc3339(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        snippets: snippets.clone(),
        groups: groups.clone(),
        preferences: prefs,
    };

    let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    let filename = format!("backup_{}.json", Utc::now().format("%Y%m%d_%H%M%S"));
    let path = backup_dir().join(&filename);

    fs::write(&path, &json).map_err(|e| format!("Failed to write backup: {}", e))?;

    Ok(BackupInfo {
        filename,
        created_at: data.created_at,
        snippet_count: snippets.len(),
        group_count: groups.len(),
        size_bytes: json.len() as u64,
    })
}

/// List all backups
pub fn list_backups() -> Result<Vec<BackupInfo>, String> {
    let dir = backup_dir();
    let mut backups = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(data) = serde_json::from_str::<BackupData>(&content) {
                        let meta = fs::metadata(&path).ok();
                        backups.push(BackupInfo {
                            filename: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                            created_at: data.created_at,
                            snippet_count: data.snippets.len(),
                            group_count: data.groups.len(),
                            size_bytes: meta.map(|m| m.len()).unwrap_or(0),
                        });
                    }
                }
            }
        }
    }

    // Sort by newest first
    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(backups)
}

/// Restore from a backup file
pub fn restore_backup(filename: &str) -> Result<(usize, usize), String> {
    if filename.contains("..") {
        return Err("Invalid filename".to_string());
    }
    let path = backup_dir().join(filename);
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read backup: {}", e))?;

    let data: BackupData = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid backup format: {}", e))?;

    // Clear existing data first
    store::clear_all_data()?;

    // Restore groups
    let mut group_count = 0;
    for g in &data.groups {
        if store::add_group(g).is_ok() {
            group_count += 1;
        }
    }

    // Restore snippets
    let mut snippet_count = 0;
    for s in &data.snippets {
        if store::add_snippet(s).is_ok() {
            snippet_count += 1;
        }
    }

    // Restore preferences
    for (key, value) in &data.preferences {
        let _ = store::set_preference(key, value);
    }

    Ok((snippet_count, group_count))
}

/// Delete a backup file
pub fn delete_backup(filename: &str) -> Result<(), String> {
    let path = backup_dir().join(filename);
    fs::remove_file(&path).map_err(|e| format!("Failed to delete backup: {}", e))
}

/// Restore from raw JSON content (e.g. from file picker)
pub fn restore_from_json(json_content: &str) -> Result<(usize, usize), String> {
    let data: BackupData = serde_json::from_str(json_content)
        .map_err(|e| format!("Invalid backup format: {}", e))?;

    store::clear_all_data()?;

    let mut group_count = 0;
    for g in &data.groups {
        if store::add_group(g).is_ok() { group_count += 1; }
    }

    let mut snippet_count = 0;
    for s in &data.snippets {
        if store::add_snippet(s).is_ok() { snippet_count += 1; }
    }

    for (key, value) in &data.preferences {
        let _ = store::set_preference(key, value);
    }

    Ok((snippet_count, group_count))
}
