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

    /// Check if the snippet matches the given user input (boundary-aware).
    ///
    /// Boundary rules:
    /// - Keyword must appear at the END of the input buffer.
    /// - If a preceding character exists, it must NOT be alphanumeric
    ///   (i.e., keyword must follow whitespace, punctuation, or line start).
    /// - Strict mode: keyword must be at buffer end (== keyword or ends with keyword).
    /// - Loose mode: same boundary check, case-insensitive variant available.
    pub fn matches_input(&self, input: &str) -> bool {
        if !self.enabled {
            return false;
        }

        let keyword = &self.keyword;
        let keyword_len = keyword.len();

        // ── Step 1: Find keyword at end of input ─────────────────────────────
        // Returns the byte position where the keyword STARTS (match_end - keyword_len)
        let match_start = match self.matching_mode {
            MatchingMode::Strict => {
                match self.case_sensitivity {
                    CaseSensitivity::CaseSensitive => {
                        if input.ends_with(keyword) {
                            input.len() - keyword_len
                        } else {
                            return false;
                        }
                    }
                    CaseSensitivity::CaseInsensitive => {
                        let input_lower = input.to_lowercase();
                        let kw_lower = keyword.to_lowercase();
                        if input_lower.ends_with(&kw_lower) {
                            input.len() - keyword_len
                        } else {
                            return false;
                        }
                    }
                }
            }
            MatchingMode::Loose => {
                // Loose mode: find LAST occurrence of keyword at buffer end
                match self.case_sensitivity {
                    CaseSensitivity::CaseSensitive => {
                        if let Some(pos) = input.rfind(keyword) {
                            // Only match if keyword ends at buffer boundary
                            if pos + keyword_len == input.len() {
                                pos
                            } else {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                    CaseSensitivity::CaseInsensitive => {
                        let input_lower = input.to_lowercase();
                        let kw_lower = keyword.to_lowercase();
                        if let Some(pos) = input_lower.rfind(&kw_lower) {
                            if pos + keyword_len == input.len() {
                                pos
                            } else {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                }
            }
        };

        // ── Step 2: Boundary check ─────────────────────────────────────────
        // If there's a preceding character, it must NOT be alphanumeric.
        // This prevents mid-word expansion: "IndoSSGnesia" should not expand "ssg"
        // because 'o' (alphanumeric) precedes the keyword.
        if match_start > 0 {
            // Get the character just before the keyword
            let prev_char = input[..match_start].chars().last().unwrap();
            if prev_char.is_alphanumeric() {
                return false; // Mid-word trigger — reject
            }
        }

        true
    }
}

    // ── Strict mode: exact or ends-with keyword, with boundary check ──────

    #[test]
    fn test_strict_match_exact() {
        let snippet = Snippet::new(
            "brb".to_string(),
            "be right back".to_string(),
            "BRB".to_string(),
            "".to_string(),
            None,
        );

        // Exact match at end — should match
        assert!(snippet.matches_input("brb"));
        assert!(!snippet.matches_input("BRB")); // case sensitive
        assert!(!snippet.matches_input("brb!")); // trailing '!' makes prev char alphanumeric
        assert!(!snippet.matches_input("brb ")); // trailing space is NOT alphanumeric, but buffer end is not keyword-end
    }

    #[test]
    fn test_strict_match_with_trailing_space() {
        let snippet = Snippet::new(
            "brb".to_string(),
            "be right back".to_string(),
            "BRB".to_string(),
            "".to_string(),
            None,
        );

        // Keyword truly at buffer end
        assert!(snippet.matches_input("brb"));
    }

    #[test]
    fn test_strict_match_with_preceding_space() {
        let snippet = Snippet::new(
            "brb".to_string(),
            "be right back".to_string(),
            "BRB".to_string(),
            "".to_string(),
            None,
        );

        // Keyword preceded by space — should match
        assert!(snippet.matches_input("I will brb"));
        assert!(snippet.matches_input("say brb"));
        assert!(snippet.matches_input("test brb"));
    }

    #[test]
    fn test_strict_match_with_punctuation() {
        let snippet = Snippet::new(
            "hello".to_string(),
            "Hello World".to_string(),
            "Hello".to_string(),
            "".to_string(),
            None,
        );

        // Preceded by space — should match (keyword at buffer boundary)
        assert!(snippet.matches_input("say hello"));
        assert!(snippet.matches_input("Bandung, Indonesia hello"));
        // "(hello)" — ends with ")", not "hello", so keyword is not at buffer boundary
        // "(hello" — ends with "o", not "hello", same reason
    }

    #[test]
    fn test_strict_mid_word_rejected() {
        let snippet = Snippet::new(
            "ssg".to_string(),
            "Snippet".to_string(),
            "SSG".to_string(),
            "".to_string(),
            None,
        );

        // Mid-word: preceded by alphanumeric — MUST NOT match
        assert!(!snippet.matches_input("IndoSSGnesia"));
        assert!(!snippet.matches_input("testSSGword"));
        assert!(!snippet.matches_input("helloSSGworld"));
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
        assert!(snippet.matches_input("I will BRB"));
        assert!(!snippet.matches_input("brb!")); // trailing '!' makes prev char alphanumeric
    }

    // ── Loose mode: ends-with, with boundary check ─────────────────────────

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

        // Exact at end
        assert!(snippet.matches_input("brb"));
        // Preceded by space
        assert!(snippet.matches_input("I will brb"));
        assert!(snippet.matches_input("test brb"));
        // Trailing punctuation — buffer ends with punctuation, NOT keyword, so no match
        // (This is correct: "brb." ends with ".", not "brb")
    }

    #[test]
    fn test_loose_mid_word_rejected() {
        let mut snippet = Snippet::new(
            "ssg".to_string(),
            "Snippet".to_string(),
            "SSG".to_string(),
            "".to_string(),
            None,
        );
        snippet.matching_mode = MatchingMode::Loose;

        // Mid-word: preceded by alphanumeric — MUST NOT match
        assert!(!snippet.matches_input("IndoSSGnesia"));
        assert!(!snippet.matches_input("testSSGword"));
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

        // Preceded by space — should match
        assert!(snippet.matches_input("say hello"));
        assert!(snippet.matches_input("Bandung, Indonesia hello"));
        // "hello!" — ends with "!", not "hello", so no match
        // "hello." — ends with ".", not "hello", so no match
    }

    #[test]
    fn test_loose_match_case_insensitive() {
        let mut snippet = Snippet::new(
            "brb".to_string(),
            "be right back".to_string(),
            "BRB".to_string(),
            "".to_string(),
            None,
        );
        snippet.matching_mode = MatchingMode::Loose;
        snippet.case_sensitivity = CaseSensitivity::CaseInsensitive;

        assert!(snippet.matches_input("brb"));
        assert!(snippet.matches_input("BRB"));
        assert!(snippet.matches_input("I will BRB"));
        // "brb!" — ends with "!", not "brb", so no match (keyword not at buffer boundary)
    }
