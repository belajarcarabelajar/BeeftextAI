use crate::snippet::Snippet;
use crate::trigger;
use rusqlite::{Connection, params};
use parking_lot::Mutex;
use once_cell::sync::Lazy;
use crate::group::Group;

static DB: Lazy<Mutex<Option<Connection>>> = Lazy::new(|| Mutex::new(None));

/// Write-behind channel for non-urgent last_used_at updates (H6 fix).
/// Sending here is fire-and-forget; a background thread drains the queue.
static LAST_USED_SENDER: Lazy<std::sync::Mutex<Option<std::sync::mpsc::SyncSender<(String, String)>>>> =
    Lazy::new(|| std::sync::Mutex::new(None));

/// Start the write-behind background thread for last_used_at updates.
/// Must be called once after init_db().
pub fn start_last_used_writer() {
    let (tx, rx) = std::sync::mpsc::sync_channel::<(String, String)>(256);
    *LAST_USED_SENDER.lock().unwrap() = Some(tx);
    std::thread::spawn(move || {
        for (uuid, timestamp) in rx {
            let guard = DB.lock();
            if let Some(conn) = guard.as_ref() {
                let _ = conn.execute(
                    "UPDATE snippets SET last_used_at = ?1 WHERE uuid = ?2",
                    params![timestamp, uuid],
                );
            }
        }
    });
}

/// Non-blocking enqueue of a last_used_at update.
/// Drops the update silently if the channel is full (256 slots) — acceptable for usage stats.
pub fn async_update_last_used(uuid: &str, timestamp: &str) {
    if let Ok(guard) = LAST_USED_SENDER.lock() {
        if let Some(sender) = guard.as_ref() {
            let _ = sender.try_send((uuid.to_string(), timestamp.to_string()));
        }
    }
}


/// Initialize the SQLite database
pub fn init_db(db_path: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;

    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS groups (
            uuid TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT DEFAULT '',
            enabled INTEGER DEFAULT 1,
            created_at TEXT NOT NULL,
            modified_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS snippets (
            uuid TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            keyword TEXT NOT NULL,
            snippet TEXT NOT NULL,
            description TEXT DEFAULT '',
            matching_mode TEXT DEFAULT 'Strict',
            case_sensitivity TEXT DEFAULT 'CaseSensitive',
            group_id TEXT,
            enabled INTEGER DEFAULT 1,
            created_at TEXT NOT NULL,
            modified_at TEXT NOT NULL,
            last_used_at TEXT,
            ai_generated INTEGER DEFAULT 0,
            embedding BLOB,
            image_data TEXT,
            content_type TEXT DEFAULT 'Text',
            FOREIGN KEY (group_id) REFERENCES groups(uuid)
        );

        CREATE TABLE IF NOT EXISTS preferences (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS chat_history (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
    ").map_err(|e| e.to_string())?;

    // L5: Enable WAL mode for better concurrent read/write performance.
    // WAL allows readers to not block writers and vice versa.
    conn.execute_batch("PRAGMA journal_mode=WAL;").map_err(|e| e.to_string())?;


    // Migration: add image_data and content_type columns to existing snippets table
    let has_image_data: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('snippets') WHERE name='image_data'",
        [],
        |row| row.get(0)
    ).unwrap_or(false);
    if !has_image_data {
        conn.execute("ALTER TABLE snippets ADD COLUMN image_data TEXT", [])
            .map_err(|e| e.to_string())?;
    }

    let has_content_type: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM pragma_table_info('snippets') WHERE name='content_type'",
        [],
        |row| row.get(0)
    ).unwrap_or(false);
    if !has_content_type {
        conn.execute("ALTER TABLE snippets ADD COLUMN content_type TEXT DEFAULT 'Text'", [])
            .map_err(|e| e.to_string())?;
    }

    // M7 migration: create unique index on keyword if it doesn't exist yet.
    // Uses IF NOT EXISTS so this is idempotent across restarts.
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_snippets_keyword ON snippets(keyword);"
    ).map_err(|e| e.to_string())?;

    *DB.lock() = Some(conn);
    Ok(())
}

/// Get all snippets
pub fn get_all_snippets() -> Result<Vec<Snippet>, String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    
    let mut stmt = conn.prepare(
        "SELECT uuid, name, keyword, snippet, description, matching_mode, case_sensitivity,
                group_id, enabled, created_at, modified_at, last_used_at, ai_generated,
                image_data, content_type
         FROM snippets ORDER BY modified_at DESC"
    ).map_err(|e| e.to_string())?;

    let results = stmt.query_map([], |row| {
        let mm_str: String = row.get(5)?;
        let cs_str: String = row.get(6)?;
        let ct_str: String = row.get(14)?;
        Ok(Snippet {
            uuid: row.get(0)?,
            name: row.get(1)?,
            keyword: row.get(2)?,
            snippet: row.get(3)?,
            description: row.get(4)?,
            matching_mode: if mm_str == "Loose" { crate::snippet::MatchingMode::Loose } else { crate::snippet::MatchingMode::Strict },
            case_sensitivity: if cs_str == "CaseInsensitive" { crate::snippet::CaseSensitivity::CaseInsensitive } else { crate::snippet::CaseSensitivity::CaseSensitive },
            group_id: row.get(7)?,
            enabled: row.get::<_, i32>(8)? != 0,
            created_at: row.get(9)?,
            modified_at: row.get(10)?,
            last_used_at: row.get(11)?,
            ai_generated: row.get::<_, i32>(12)? != 0,
            image_data: row.get(13)?,
            content_type: match ct_str.as_str() {
                "Image" => crate::snippet::ContentType::Image,
                "Both" => crate::snippet::ContentType::Both,
                _ => crate::snippet::ContentType::Text,
            },
        })
    }).map_err(|e| e.to_string())?;

    results.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// P16: Get only snippets that are eligible for trigger matching.
/// Filters: enabled=1, keyword is not empty, content_type is Text or Both.
/// This is used by the trigger cache to avoid loading image-only or disabled snippets.
pub fn get_trigger_snippets() -> Result<Vec<Snippet>, String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    
    let mut stmt = conn.prepare(
        "SELECT uuid, name, keyword, snippet, description, matching_mode, case_sensitivity,
                group_id, enabled, created_at, modified_at, last_used_at, ai_generated,
                image_data, content_type
         FROM snippets
         WHERE enabled = 1 AND keyword != '' AND content_type != 'Image'
         ORDER BY modified_at DESC"
    ).map_err(|e| e.to_string())?;

    let results = stmt.query_map([], |row| {
        let mm_str: String = row.get(5)?;
        let cs_str: String = row.get(6)?;
        let ct_str: String = row.get(14)?;
        Ok(Snippet {
            uuid: row.get(0)?,
            name: row.get(1)?,
            keyword: row.get(2)?,
            snippet: row.get(3)?,
            description: row.get(4)?,
            matching_mode: if mm_str == "Loose" { crate::snippet::MatchingMode::Loose } else { crate::snippet::MatchingMode::Strict },
            case_sensitivity: if cs_str == "CaseInsensitive" { crate::snippet::CaseSensitivity::CaseInsensitive } else { crate::snippet::CaseSensitivity::CaseSensitive },
            group_id: row.get(7)?,
            enabled: row.get::<_, i32>(8)? != 0,
            created_at: row.get(9)?,
            modified_at: row.get(10)?,
            last_used_at: row.get(11)?,
            ai_generated: row.get::<_, i32>(12)? != 0,
            image_data: row.get(13)?,
            content_type: match ct_str.as_str() {
                "Image" => crate::snippet::ContentType::Image,
                "Both" => crate::snippet::ContentType::Both,
                _ => crate::snippet::ContentType::Text,
            },
        })
    }).map_err(|e| e.to_string())?;

    results.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Add a new snippet
pub fn add_snippet(s: &Snippet) -> Result<(), String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    let mm = format!("{:?}", s.matching_mode);
    let cs = format!("{:?}", s.case_sensitivity);
    let ct = format!("{:?}", s.content_type);

    // M7: validate keyword uniqueness before insert
    let existing: i64 = conn.query_row(
        "SELECT COUNT(*) FROM snippets WHERE keyword = ?1",
        params![s.keyword],
        |row| row.get(0),
    ).unwrap_or(0);
    if existing > 0 {
        return Err(format!("A snippet with keyword '{}' already exists.", s.keyword));
    }

    conn.execute(
        "INSERT INTO snippets (uuid, name, keyword, snippet, description, matching_mode, case_sensitivity, group_id, enabled, created_at, modified_at, last_used_at, ai_generated, image_data, content_type)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![s.uuid, s.name, s.keyword, s.snippet, s.description, mm, cs, s.group_id, s.enabled as i32, s.created_at, s.modified_at, s.last_used_at, s.ai_generated as i32, s.image_data, ct],
    ).map_err(|e| e.to_string())?;
    drop(guard);
    trigger::invalidate_cache();
    Ok(())
}

/// Update an existing snippet
pub fn update_snippet(s: &Snippet) -> Result<(), String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    let mm = format!("{:?}", s.matching_mode);
    let cs = format!("{:?}", s.case_sensitivity);
    let ct = format!("{:?}", s.content_type);

    // M7: validate keyword uniqueness — allow same snippet to keep its own keyword
    let conflict: i64 = conn.query_row(
        "SELECT COUNT(*) FROM snippets WHERE keyword = ?1 AND uuid != ?2",
        params![s.keyword, s.uuid],
        |row| row.get(0),
    ).unwrap_or(0);
    if conflict > 0 {
        return Err(format!("Another snippet already uses keyword '{}'.", s.keyword));
    }

    conn.execute(
        "UPDATE snippets SET name=?1, keyword=?2, snippet=?3, description=?4, matching_mode=?5, case_sensitivity=?6, group_id=?7, enabled=?8, modified_at=?9, last_used_at=?10, ai_generated=?11, image_data=?12, content_type=?13 WHERE uuid=?14",
        params![s.name, s.keyword, s.snippet, s.description, mm, cs, s.group_id, s.enabled as i32, s.modified_at, s.last_used_at, s.ai_generated as i32, s.image_data, ct, s.uuid],
    ).map_err(|e| e.to_string())?;
    drop(guard);
    trigger::invalidate_cache();
    Ok(())
}

/// Delete a snippet
pub fn delete_snippet(uuid: &str) -> Result<(), String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    conn.execute("DELETE FROM snippets WHERE uuid = ?1", params![uuid]).map_err(|e| e.to_string())?;
    drop(guard);
    trigger::invalidate_cache();
    Ok(())
}

/// Get all groups
pub fn get_all_groups() -> Result<Vec<Group>, String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    let mut stmt = conn.prepare(
        "SELECT uuid, name, description, enabled, created_at, modified_at FROM groups ORDER BY name"
    ).map_err(|e| e.to_string())?;
    let results = stmt.query_map([], |row| {
        Ok(Group {
            uuid: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            enabled: row.get::<_, i32>(3)? != 0,
            created_at: row.get(4)?,
            modified_at: row.get(5)?,
        })
    }).map_err(|e| e.to_string())?;
    results.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Add a new group
pub fn add_group(g: &Group) -> Result<(), String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    conn.execute(
        "INSERT INTO groups (uuid, name, description, enabled, created_at, modified_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![g.uuid, g.name, g.description, g.enabled as i32, g.created_at, g.modified_at],
    ).map_err(|e| e.to_string())?;
    drop(guard);
    trigger::invalidate_cache();
    Ok(())
}

/// Update a group
pub fn update_group(g: &Group) -> Result<(), String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    conn.execute(
        "UPDATE groups SET name=?1, description=?2, enabled=?3, modified_at=?4 WHERE uuid=?5",
        params![g.name, g.description, g.enabled as i32, g.modified_at, g.uuid],
    ).map_err(|e| e.to_string())?;
    drop(guard);
    trigger::invalidate_cache();
    Ok(())
}

/// Delete a group and optionally all snippets inside it
pub fn delete_group(uuid: &str, delete_snippets: bool) -> Result<(), String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    if delete_snippets {
        conn.execute("DELETE FROM snippets WHERE group_id = ?1", params![uuid]).map_err(|e| e.to_string())?;
    } else {
        // Move snippets to ungrouped
        conn.execute("UPDATE snippets SET group_id = NULL WHERE group_id = ?1", params![uuid]).map_err(|e| e.to_string())?;
    }
    conn.execute("DELETE FROM groups WHERE uuid = ?1", params![uuid]).map_err(|e| e.to_string())?;
    drop(guard);
    trigger::invalidate_cache();
    Ok(())
}

/// Delete all snippets in a specific group
pub fn delete_snippets_in_group(group_uuid: &str) -> Result<usize, String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    let count = conn.execute("DELETE FROM snippets WHERE group_id = ?1", params![group_uuid]).map_err(|e| e.to_string())?;
    drop(guard);
    trigger::invalidate_cache();
    Ok(count)
}

/// Save an embedding for a snippet
pub fn save_embedding(snippet_uuid: &str, embedding: &[f32]) -> Result<(), String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    let bytes: Vec<u8> = embedding.iter().flat_map(|f| f.to_le_bytes()).collect();
    conn.execute(
        "UPDATE snippets SET embedding = ?1 WHERE uuid = ?2",
        params![bytes, snippet_uuid],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Get all snippet embeddings for semantic search
pub fn get_all_embeddings() -> Result<Vec<(String, Vec<f32>)>, String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    let mut stmt = conn.prepare(
        "SELECT uuid, embedding FROM snippets WHERE embedding IS NOT NULL"
    ).map_err(|e| e.to_string())?;
    let results = stmt.query_map([], |row| {
        let uuid: String = row.get(0)?;
        let bytes: Vec<u8> = row.get(1)?;
        let embedding: Vec<f32> = bytes.chunks(4)
            .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        Ok((uuid, embedding))
    }).map_err(|e| e.to_string())?;
    results.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Save a chat message
pub fn save_chat_message(role: &str, content: &str) -> Result<(), String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    conn.execute(
        "INSERT INTO chat_history (role, content, created_at) VALUES (?1, ?2, ?3)",
        params![role, content, chrono::Utc::now().to_rfc3339()],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Get recent chat history
pub fn get_chat_history(limit: i64) -> Result<Vec<(String, String)>, String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    let mut stmt = conn.prepare(
        "SELECT role, content FROM chat_history ORDER BY id DESC LIMIT ?1"
    ).map_err(|e| e.to_string())?;
    let results = stmt.query_map(params![limit], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }).map_err(|e| e.to_string())?;
    let mut msgs: Vec<_> = results.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    msgs.reverse();
    Ok(msgs)
}

/// Clear chat history
pub fn clear_chat_history() -> Result<(), String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    conn.execute("DELETE FROM chat_history", []).map_err(|e| e.to_string())?;
    Ok(())
}

/// Get a preference value
pub fn get_preference(key: &str) -> Result<Option<String>, String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    let mut stmt = conn.prepare("SELECT value FROM preferences WHERE key = ?1").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![key]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(Some(row.get(0).map_err(|e| e.to_string())?))
    } else {
        Ok(None)
    }
}

/// Set a preference value
pub fn set_preference(key: &str, value: &str) -> Result<(), String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    conn.execute(
        "INSERT OR REPLACE INTO preferences (key, value) VALUES (?1, ?2)",
        params![key, value],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Toggle snippet enabled/disabled
pub fn toggle_snippet_enabled(uuid: &str, enabled: bool) -> Result<(), String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    conn.execute(
        "UPDATE snippets SET enabled = ?1, modified_at = ?2 WHERE uuid = ?3",
        params![enabled as i32, chrono::Utc::now().to_rfc3339(), uuid],
    ).map_err(|e| e.to_string())?;
    drop(guard);
    trigger::invalidate_cache();
    Ok(())
}

/// Get snippet count by group
pub fn get_snippet_count_by_group() -> Result<Vec<(Option<String>, i64)>, String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    let mut stmt = conn.prepare(
        "SELECT group_id, COUNT(*) FROM snippets GROUP BY group_id"
    ).map_err(|e| e.to_string())?;
    let results = stmt.query_map([], |row| {
        Ok((row.get::<_, Option<String>>(0)?, row.get::<_, i64>(1)?))
    }).map_err(|e| e.to_string())?;
    results.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Get all preferences
pub fn get_all_preferences() -> Result<Vec<(String, String)>, String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    let mut stmt = conn.prepare("SELECT key, value FROM preferences")
        .map_err(|e| e.to_string())?;
    let results = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }).map_err(|e| e.to_string())?;
    results.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Clear all data (for restore)
pub fn clear_all_data() -> Result<(), String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    conn.execute_batch("
        DELETE FROM snippets;
        DELETE FROM groups;
        DELETE FROM chat_history;
    ").map_err(|e| e.to_string())?;
    drop(guard);
    trigger::invalidate_cache();
    Ok(())
}

/// Get snippet usage stats
pub fn get_snippet_stats() -> Result<(i64, i64, i64, i64), String> {
    let guard = DB.lock();
    let conn = guard.as_ref().ok_or("Database not initialized")?;
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM snippets", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let enabled: i64 = conn.query_row("SELECT COUNT(*) FROM snippets WHERE enabled = 1", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let ai_count: i64 = conn.query_row("SELECT COUNT(*) FROM snippets WHERE ai_generated = 1", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let embedded: i64 = conn.query_row("SELECT COUNT(*) FROM snippets WHERE embedding IS NOT NULL", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok((total, enabled, ai_count, embedded))
}
