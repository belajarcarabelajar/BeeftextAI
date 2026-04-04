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
    /// Mirrors original Beeftext's Combo::matchesForInput() behavior:
    /// - Strict mode: exact match only (input.compare(keyword_) == 0)
    /// - Loose mode: ends with (input.endsWith(keyword_)), no word boundary check
    pub fn matches_input(&self, input: &str) -> bool {
        if !self.enabled {
            return false;
        }
        match self.matching_mode {
            MatchingMode::Strict => {
                // Original Beeftext strict mode: exact match only, no word boundary
                match self.case_sensitivity {
                    CaseSensitivity::CaseSensitive => input == self.keyword,
                    CaseSensitivity::CaseInsensitive => input.eq_ignore_ascii_case(&self.keyword),
                }
            }
            MatchingMode::Loose => {
                // Original Beeftext loose mode: endsWith only, no boundary check
                match self.case_sensitivity {
                    CaseSensitivity::CaseSensitive => input.ends_with(&self.keyword),
                    CaseSensitivity::CaseInsensitive => input.to_lowercase().ends_with(&self.keyword.to_lowercase()),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strict_match_exact() {
        let snippet = Snippet::new(
            "brb".to_string(),
            "be right back".to_string(),
            "BRB".to_string(),
            "".to_string(),
            None,
        );

        // Exact match only - case sensitive
        assert!(snippet.matches_input("brb"));
        assert!(!snippet.matches_input("BRB")); // case sensitive
        assert!(!snippet.matches_input("brb!")); // not exact match
        assert!(!snippet.matches_input("brb ")); // not exact match
        assert!(!snippet.matches_input("I will brb")); // not exact match
    }

    #[test]
    fn test_strict_match_case_insensitive() {
        let mut snippet = Snippet::new(
            "brb".to_string(),
            "be right back".to_string(),
            "BRB".to_string(),
            "".to_string(),
            None,
        );
        snippet.case_sensitivity = CaseSensitivity::CaseInsensitive;

        assert!(snippet.matches_input("brb"));
        assert!(snippet.matches_input("BRB"));
        assert!(snippet.matches_input("BrB"));
        assert!(!snippet.matches_input("brb!")); // not exact match
    }

    #[test]
    fn test_loose_match_endswith() {
        let mut snippet = Snippet::new(
            "brb".to_string(),
            "be right back".to_string(),
            "BRB".to_string(),
            "".to_string(),
            None,
        );
        snippet.matching_mode = MatchingMode::Loose;

        // Loose mode: ends with, no word boundary
        assert!(snippet.matches_input("brb"));
        assert!(snippet.matches_input("I will brb"));
        assert!(snippet.matches_input("brb!")); // ends with brb, no boundary check
        assert!(snippet.matches_input("test brb"));
    }

    #[test]
    fn test_loose_match_with_punctuation() {
        let mut snippet = Snippet::new(
            "hello".to_string(),
            "Hello World".to_string(),
            "Hello".to_string(),
            "".to_string(),
            None,
        );
        snippet.matching_mode = MatchingMode::Loose;

        // No word boundary check - triggers on any ending
        assert!(snippet.matches_input("say hello"));
        assert!(snippet.matches_input("hello!"));
        assert!(snippet.matches_input("hello."));
    }
}
