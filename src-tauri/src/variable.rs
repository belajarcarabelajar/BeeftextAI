use chrono::Local;
use crate::ollama::OllamaClient;
use regex::Regex;

/// Evaluate all template variables in a snippet text
/// Variables: #{clipboard}, #{date}, #{time}, #{dateTime:format}, #{input:desc},
///            #{combo:keyword}, #{envVar:name}, #{ai:prompt},
///            #{upper:text}, #{lower:text}, #{trim:text}
pub async fn evaluate_variables(text: &str, ollama: &OllamaClient) -> String {
    let mut result = text.to_string();

    // #{clipboard} — current clipboard content
    if result.contains("#{clipboard}") {
        let clip_text = arboard::Clipboard::new()
            .and_then(|mut c| c.get_text())
            .unwrap_or_default();
        result = result.replace("#{clipboard}", &clip_text);
    }

    // #{date} — current date
    result = result.replace("#{date}", &Local::now().format("%Y-%m-%d").to_string());

    // #{time} — current time
    result = result.replace("#{time}", &Local::now().format("%H:%M:%S").to_string());

    // #{dateTime:format} — custom date/time format
    let dt_re = Regex::new(r"#\{dateTime:([^}]+)\}").unwrap();
    let dt_result = result.clone();
    for cap in dt_re.captures_iter(&dt_result) {
        let full_match = &cap[0];
        let format = &cap[1];
        let formatted = Local::now().format(format).to_string();
        result = result.replace(full_match, &formatted);
    }

    // #{envVar:name} — environment variable
    let env_re = Regex::new(r"#\{envVar:([^}]+)\}").unwrap();
    let env_result = result.clone();
    for cap in env_re.captures_iter(&env_result) {
        let full_match = &cap[0];
        let var_name = &cap[1];
        let value = std::env::var(var_name).unwrap_or_default();
        result = result.replace(full_match, &value);
    }

    // #{upper:text} — uppercase
    let upper_re = Regex::new(r"#\{upper:([^}]+)\}").unwrap();
    let upper_result = result.clone();
    for cap in upper_re.captures_iter(&upper_result) {
        let full_match = &cap[0];
        let text_val = &cap[1];
        result = result.replace(full_match, &text_val.to_uppercase());
    }

    // #{lower:text} — lowercase
    let lower_re = Regex::new(r"#\{lower:([^}]+)\}").unwrap();
    let lower_result = result.clone();
    for cap in lower_re.captures_iter(&lower_result) {
        let full_match = &cap[0];
        let text_val = &cap[1];
        result = result.replace(full_match, &text_val.to_lowercase());
    }

    // #{trim:text} — trim whitespace
    let trim_re = Regex::new(r"#\{trim:([^}]+)\}").unwrap();
    let trim_result = result.clone();
    for cap in trim_re.captures_iter(&trim_result) {
        let full_match = &cap[0];
        let text_val = &cap[1];
        result = result.replace(full_match, text_val.trim());
    }

    // #{ai:prompt} — generate text via Ollama
    let ai_re = Regex::new(r"#\{ai:([^}]+)\}").unwrap();
    let ai_result = result.clone();
    for cap in ai_re.captures_iter(&ai_result) {
        let full_match = &cap[0];
        let prompt = &cap[1];
        match ollama.generate(prompt, None).await {
            Ok(ai_text) => {
                result = result.replace(full_match, &ai_text);
            }
            Err(e) => {
                eprintln!("AI variable error: {}", e);
                result = result.replace(full_match, &format!("[AI Error: {}]", e));
            }
        }
    }

    // #{combo:keyword} — reference another snippet
    let combo_re = Regex::new(r"#\{combo:([^}]+)\}").unwrap();
    let combo_result = result.clone();
    for cap in combo_re.captures_iter(&combo_result) {
        let full_match = &cap[0];
        let keyword = &cap[1];
        // Look up the referenced snippet
        if let Ok(snippets) = crate::store::get_all_snippets() {
            if let Some(referenced) = snippets.iter().find(|s| s.keyword == keyword) {
                result = result.replace(full_match, &referenced.snippet);
            }
        }
    }

    result
}
