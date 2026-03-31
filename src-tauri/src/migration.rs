use serde::{Deserialize, Serialize};
use crate::snippet::{Snippet, MatchingMode, CaseSensitivity};
use crate::group::Group;
use crate::store;
use chrono::Utc;

/// Beeftext JSON format structures (v10)
#[derive(Deserialize, Debug)]
struct BeeftextFile {
    #[serde(default)]
    combos: Vec<BeeftextCombo>,
    #[serde(default)]
    groups: Vec<BeeftextGroup>,
}

#[derive(Deserialize, Debug)]
struct BeeftextCombo {
    #[serde(default)]
    name: String,
    #[serde(default)]
    keyword: String,
    #[serde(default)]
    snippet: String,
    #[serde(default, alias = "substitutionShortcut")]
    description: String,
    #[serde(default)]
    group: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default, alias = "matchingMode")]
    matching_mode: Option<i32>,
    #[serde(default, alias = "caseSensitivity")]
    case_sensitivity: Option<i32>,
}

#[derive(Deserialize, Debug)]
struct BeeftextGroup {
    #[serde(default)]
    uuid: Option<String>,
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
}

#[derive(Serialize)]
pub struct ImportResult {
    pub snippets_imported: usize,
    pub groups_imported: usize,
    pub errors: Vec<String>,
}

/// Import from Beeftext JSON file
pub fn import_beeftext_json(json_content: &str) -> Result<ImportResult, String> {
    let data: BeeftextFile = serde_json::from_str(json_content)
        .map_err(|e| format!("Failed to parse Beeftext JSON: {}", e))?;
    
    let mut result = ImportResult {
        snippets_imported: 0,
        groups_imported: 0,
        errors: Vec::new(),
    };

    // Import groups first
    let mut group_uuid_map: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    
    for bg in &data.groups {
        let g = Group::new(bg.name.clone(), bg.description.clone());
        let old_uuid = bg.uuid.clone().unwrap_or_default();
        
        match store::add_group(&g) {
            Ok(_) => {
                group_uuid_map.insert(old_uuid, g.uuid.clone());
                result.groups_imported += 1;
            }
            Err(e) => result.errors.push(format!("Group '{}': {}", bg.name, e)),
        }
    }

    // Import combos / snippets
    for bc in &data.combos {
        if bc.keyword.is_empty() && bc.snippet.is_empty() {
            continue;
        }

        let group_id = bc.group.as_ref().and_then(|old_id| {
            group_uuid_map.get(old_id).cloned()
        });

        let matching_mode = match bc.matching_mode.unwrap_or(0) {
            1 => MatchingMode::Loose,
            _ => MatchingMode::Strict,
        };

        let case_sensitivity = match bc.case_sensitivity.unwrap_or(0) {
            1 => CaseSensitivity::CaseInsensitive,
            _ => CaseSensitivity::CaseSensitive,
        };

        let mut s = Snippet::new(
            bc.keyword.clone(),
            bc.snippet.clone(),
            bc.name.clone(),
            bc.description.clone(),
            group_id,
        );
        s.matching_mode = matching_mode;
        s.case_sensitivity = case_sensitivity;
        s.enabled = bc.enabled.unwrap_or(true);

        match store::add_snippet(&s) {
            Ok(_) => result.snippets_imported += 1,
            Err(e) => result.errors.push(format!("Snippet '{}': {}", bc.keyword, e)),
        }
    }

    Ok(result)
}

/// Export all snippets and groups as JSON
pub fn export_all_as_json() -> Result<String, String> {
    let snippets = store::get_all_snippets()?;
    let groups = store::get_all_groups()?;
    
    #[derive(Serialize)]
    struct ExportData {
        version: String,
        exported_at: String,
        groups: Vec<Group>,
        snippets: Vec<Snippet>,
    }
    
    let data = ExportData {
        version: "1.0".to_string(),
        exported_at: Utc::now().to_rfc3339(),
        groups,
        snippets,
    };
    
    serde_json::to_string_pretty(&data).map_err(|e| e.to_string())
}

/// Export snippets as CSV
pub fn export_as_csv() -> Result<String, String> {
    let snippets = store::get_all_snippets()?;
    let groups = store::get_all_groups()?;
    
    let mut csv = String::from("Keyword,Name,Snippet,Description,Group,Matching Mode,Case Sensitivity,Enabled,AI Generated\n");
    
    for s in &snippets {
        let group_name = s.group_id.as_ref()
            .and_then(|gid| groups.iter().find(|g| &g.uuid == gid))
            .map(|g| g.name.clone())
            .unwrap_or_default();
        
        let snippet_escaped = s.snippet.replace('"', "\"\"");
        let desc_escaped = s.description.replace('"', "\"\"");
        
        csv.push_str(&format!(
            "\"{}\",\"{}\",\"{}\",\"{}\",\"{}\",\"{:?}\",\"{:?}\",{},{}\n",
            s.keyword, s.name, snippet_escaped, desc_escaped, group_name,
            s.matching_mode, s.case_sensitivity, s.enabled, s.ai_generated
        ));
    }
    
    Ok(csv)
}

/// Generate a cheat sheet (HTML)
pub fn generate_cheat_sheet() -> Result<String, String> {
    let snippets = store::get_all_snippets()?;
    let groups = store::get_all_groups()?;
    
    let mut html = String::from(r#"<!DOCTYPE html>
<html><head><meta charset="UTF-8">
<title>BeefText AI — Cheat Sheet</title>
<style>
body { font-family: 'Segoe UI', sans-serif; background: #0a0e1a; color: #f1f5f9; padding: 40px; }
h1 { background: linear-gradient(135deg, #6366f1, #a78bfa); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
h2 { color: #818cf8; margin-top: 30px; }
table { width: 100%; border-collapse: collapse; margin-bottom: 20px; }
th { background: #1a2035; color: #94a3b8; text-align: left; padding: 10px; font-size: 12px; text-transform: uppercase; }
td { padding: 10px; border-bottom: 1px solid rgba(148,163,184,0.1); }
.keyword { background: rgba(99,102,241,0.15); color: #818cf8; padding: 2px 8px; border-radius: 4px; font-family: monospace; }
.ai { background: rgba(139,92,246,0.15); color: #a78bfa; padding: 1px 6px; border-radius: 10px; font-size: 10px; }
.snippet { color: #94a3b8; max-width: 400px; }
</style></head><body>
<h1>⚡ BeefText AI — Cheat Sheet</h1>
<p style="color:#64748b">Generated on "#);
    
    html.push_str(&Utc::now().format("%Y-%m-%d %H:%M").to_string());
    html.push_str(r#"</p>"#);
    
    // Group snippets by group
    let ungrouped: Vec<&Snippet> = snippets.iter().filter(|s| s.group_id.is_none()).collect();
    
    if !ungrouped.is_empty() {
        html.push_str("<h2>📁 Ungrouped</h2>");
        html.push_str(&snippets_to_html_table(&ungrouped));
    }
    
    for group in &groups {
        let group_snippets: Vec<&Snippet> = snippets.iter().filter(|s| s.group_id.as_ref() == Some(&group.uuid)).collect();
        if !group_snippets.is_empty() {
            html.push_str(&format!("<h2>📁 {}</h2>", group.name));
            if !group.description.is_empty() {
                html.push_str(&format!("<p style='color:#64748b'>{}</p>", group.description));
            }
            html.push_str(&snippets_to_html_table(&group_snippets));
        }
    }
    
    html.push_str("</body></html>");
    Ok(html)
}

fn snippets_to_html_table(snippets: &[&Snippet]) -> String {
    let mut html = String::from("<table><thead><tr><th>Keyword</th><th>Name</th><th>Snippet</th></tr></thead><tbody>");
    for s in snippets {
        let ai_badge = if s.ai_generated { " <span class='ai'>🤖 AI</span>" } else { "" };
        let snippet_preview = if s.snippet.len() > 80 {
            format!("{}...", &s.snippet[..80])
        } else {
            s.snippet.clone()
        };
        html.push_str(&format!(
            "<tr><td><span class='keyword'>{}</span></td><td>{}{}</td><td class='snippet'>{}</td></tr>",
            s.keyword, s.name, ai_badge, snippet_preview.replace('<', "&lt;").replace('>', "&gt;")
        ));
    }
    html.push_str("</tbody></table>");
    html
}
