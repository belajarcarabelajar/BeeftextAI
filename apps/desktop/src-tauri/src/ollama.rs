use serde::{Deserialize, Serialize};
use reqwest::Client;

/// Ollama API client for text generation and embeddings
#[derive(Debug, Clone)]
pub struct OllamaClient {
    client: Client,
    pub base_url: String,
    pub text_model: String,
    pub embed_model: String,
}

#[derive(Serialize)]
struct GenerateRequest {
    model: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    stream: bool,
}

#[derive(Deserialize, Debug)]
pub struct GenerateResponse {
    pub response: String,
    #[allow(dead_code)]
    pub done: bool,
}

#[derive(Serialize)]
struct EmbedRequest {
    model: String,
    input: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct EmbedResponse {
    pub embeddings: Vec<Vec<f32>>,
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: Option<u64>,
}

#[derive(Deserialize, Debug)]
struct ModelsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Deserialize, Debug)]
pub struct ChatResponse {
    pub message: ChatMessage,
    #[allow(dead_code)]
    pub done: bool,
}

impl OllamaClient {
    pub fn new(base_url: String, text_model: String, embed_model: String) -> Self {
        Self {
            client: Client::new(),
            base_url,
            text_model,
            embed_model,
        }
    }

    /// Check if Ollama is running
    pub async fn is_available(&self) -> bool {
        self.client.get(&self.base_url).send().await.is_ok()
    }

    /// List available models
    pub async fn list_models(&self) -> Result<Vec<OllamaModel>, String> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self.client.get(&url).send().await.map_err(|e| e.to_string())?;
        let data: ModelsResponse = resp.json().await.map_err(|e| e.to_string())?;
        Ok(data.models)
    }

    /// Generate text using Ollama
    pub async fn generate(&self, prompt: &str, system: Option<&str>) -> Result<String, String> {
        let url = format!("{}/api/generate", self.base_url);
        let req = GenerateRequest {
            model: self.text_model.clone(),
            prompt: prompt.to_string(),
            system: system.map(|s| s.to_string()),
            stream: false,
        };
        let resp = self.client.post(&url).json(&req).send().await.map_err(|e| e.to_string())?;
        let data: GenerateResponse = resp.json().await.map_err(|e| e.to_string())?;
        Ok(data.response)
    }

    /// Chat with Ollama (multi-turn conversation)
    pub async fn chat(&self, messages: Vec<ChatMessage>) -> Result<ChatMessage, String> {
        let url = format!("{}/api/chat", self.base_url);
        let req = ChatRequest {
            model: self.text_model.clone(),
            messages,
            stream: false,
        };
        let resp = self.client.post(&url).json(&req).send().await.map_err(|e| e.to_string())?;
        let data: ChatResponse = resp.json().await.map_err(|e| e.to_string())?;
        Ok(data.message)
    }

    /// Generate embeddings for texts
    pub async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>, String> {
        let url = format!("{}/api/embed", self.base_url);
        let req = EmbedRequest {
            model: self.embed_model.clone(),
            input: texts,
        };
        let resp = self.client.post(&url).json(&req).send().await.map_err(|e| e.to_string())?;

        // Check if response is OK before trying to parse JSON
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Ollama API error (status {}): {}", status, body));
        }

        let data: EmbedResponse = resp.json().await.map_err(|e| e.to_string())?;
        Ok(data.embeddings)
    }
}
