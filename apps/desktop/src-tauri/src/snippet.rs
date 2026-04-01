use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

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

/// Content type for snippet — text only, image only, or both
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ContentType {
    Text,
    Image,
    Both,
}

impl Default for ContentType {
    fn default() -> Self { Self::Text }
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
    pub image_data: Option<String>,
    pub content_type: ContentType,
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
            image_data: None,
            content_type: ContentType::default(),
        }
    }

    /// Check if the snippet matches the given user input
    pub fn matches_input(&self, input: &str) -> bool {
        if !self.enabled {
            return false;
        }
        match self.matching_mode {
            MatchingMode::Strict => {
                let matches = match self.case_sensitivity {
                    CaseSensitivity::CaseSensitive => input.ends_with(&self.keyword),
                    CaseSensitivity::CaseInsensitive => input.to_lowercase().ends_with(&self.keyword.to_lowercase()),
                };

                // Word boundary check:
                // Trigger only if keyword is at the start of input or preceded by a non-alphanumeric character.
                if matches && input.len() > self.keyword.len() {
                    let input_chars: Vec<char> = input.chars().collect();
                    let kw_char_count = self.keyword.chars().count();
                    if input_chars.len() >= kw_char_count + 1 {
                        let prev_char_idx = input_chars.len() - kw_char_count - 1;
                        if let Some(&prev_char) = input_chars.get(prev_char_idx) {
                            if prev_char.is_alphanumeric() {
                                return false; // In the middle of a word
                            }
                        }
                    }
                }
                matches
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
