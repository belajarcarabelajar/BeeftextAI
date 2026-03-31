/// Token counting utilities using approximate character-based estimation.
/// For exact counting, cl100k_base tokenizer would be needed.
/// Approximation: ~4 chars per token (typical for English-heavy text).
const CHARS_PER_TOKEN: usize = 4;

/// Estimate token count for a string using character-based approximation.
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    // Count words
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return (text.chars().count() + CHARS_PER_TOKEN - 1) / CHARS_PER_TOKEN;
    }
    // Word-based estimate: words * 1.5 ≈ tokens
    let word_based = (words.len() * 3 + 1) / 2;
    let char_based = (text.chars().count() + CHARS_PER_TOKEN - 1) / CHARS_PER_TOKEN;
    word_based.max(char_based)
}

/// Truncate a string to approximately max_tokens.
pub fn truncate_to_tokens(text: &str, max_tokens: usize) -> String {
    if max_tokens == 0 {
        return String::new();
    }
    if estimate_tokens(text) <= max_tokens {
        return text.to_string();
    }
    // Take roughly max_tokens * 4 chars
    let max_chars = max_tokens * CHARS_PER_TOKEN;
    let mut result: String = text.chars().take(max_chars).collect();
    // Try to end at word boundary
    if !result.ends_with(' ') {
        if let Some(last_space) = result.rfind(' ') {
            result.truncate(last_space);
        }
    }
    result
}

/// Log token stats for a text with a label.
pub fn log_stats(label: &str, text: &str) {
    let chars = text.chars().count();
    let tokens = estimate_tokens(text);
    eprintln!("[TOKEN] {} | chars: {} | tokens: ~{}", label, chars, tokens);
}
