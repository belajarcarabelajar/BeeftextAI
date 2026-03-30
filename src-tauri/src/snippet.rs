use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

/// Matching mode — mirrors Beeftext's EMatchingMode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MatchingMode {
    Strict,
    Loose,
}

impl Default for MatchingMode {
    fn default() -> Self { Self::Strict }
}

/// Case sensitivity — mirrors Beeftext's ECaseSensitivity
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum CaseSensitivity {
    CaseSensitive,
    CaseInsensitive,
}

impl Default for CaseSensitivity {
    fn default() -> Self { Self::CaseSensitive }
}

/// Snippet model — modern equivalent of Beeftext's Combo class
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snippet {
    pub uuid: String,
    pub name: String,
    pub keyword: String,
    pub snippet: String,
    pub description: String,
    pub matching_mode: MatchingMode,
    pub case_sensitivity: CaseSensitivity,
    pub group_id: Option<String>,
    pub enabled: bool,
    pub created_at: String,
    pub modified_at: String,
    pub last_used_at: Option<String>,
    pub ai_generated: bool,
}

impl Snippet {
    pub fn new(keyword: String, snippet: String, name: String, description: String, group_id: Option<String>) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            uuid: Uuid::new_v4().to_string(),
            name,
            keyword,
            snippet,
            description,
            matching_mode: MatchingMode::default(),
            case_sensitivity: CaseSensitivity::default(),
            group_id,
            enabled: true,
            created_at: now.clone(),
            modified_at: now,
            last_used_at: None,
            ai_generated: false,
        }
    }

    /// Check if the snippet matches the given user input
    pub fn matches_input(&self, input: &str) -> bool {
        if !self.enabled {
            return false;
        }
        match self.matching_mode {
            MatchingMode::Strict => {
                match self.case_sensitivity {
                    CaseSensitivity::CaseSensitive => input.ends_with(&self.keyword),
                    CaseSensitivity::CaseInsensitive => input.to_lowercase().ends_with(&self.keyword.to_lowercase()),
                }
            }
            MatchingMode::Loose => {
                match self.case_sensitivity {
                    CaseSensitivity::CaseSensitive => input.contains(&self.keyword),
                    CaseSensitivity::CaseInsensitive => input.to_lowercase().contains(&self.keyword.to_lowercase()),
                }
            }
        }
    }
}
