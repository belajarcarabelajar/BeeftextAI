/// Token counting utilities using approximate character-based estimation.
/// For exact counting, cl100k_base tokenizer would be needed.
/// Approximation: ~3 chars per token (more accurate for multilingual text including Indonesian).
const CHARS_PER_TOKEN: usize = 3;

/// Estimate token count for a string using character-based approximation.
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    let chars = text.chars().count();
    // Count words
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return (chars + CHARS_PER_TOKEN - 1) / CHARS_PER_TOKEN;
    }
    // Word-based estimate: words * 1.5 ≈ tokens (English)
    let word_based = (words.len() * 3 + 1) / 2;
    // Char-based is more reliable for multilingual text
    let char_based = (chars + CHARS_PER_TOKEN - 1) / CHARS_PER_TOKEN;
    // Use max to avoid underestimating (safer for truncation)
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
    // Take roughly max_tokens * 3 chars
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
