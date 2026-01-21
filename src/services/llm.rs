use rwf::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
struct LlmError(String);

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for LlmError {}

#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<LlmMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<Choice>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: LlmMessage,
}

#[derive(Debug, Deserialize)]
struct Usage {
    total_tokens: u32,
}

pub struct LlmService {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    default_model: String,
}

impl LlmService {
    pub fn new(api_key: String, base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            base_url,
            default_model: "anthropic/claude-3.5-sonnet".to_string(),
        }
    }

    pub async fn chat(
        &self,
        messages: Vec<LlmMessage>,
        system_prompt: Option<String>,
    ) -> Result<String, Error> {
        let mut final_messages = vec![];
        
        if let Some(prompt) = system_prompt {
            final_messages.push(LlmMessage {
                role: "system".to_string(),
                content: prompt,
            });
        }
        
        final_messages.extend(messages);

        let request = OpenRouterRequest {
            model: self.default_model.clone(),
            messages: final_messages,
            temperature: Some(0.7),
            max_tokens: Some(2000),
        };

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "https://balungpisah.app")
            .header("X-Title", "BalungPisah")
            .json(&request)
            .send()
            .await
            .map_err(|e| Error::new(LlmError(format!("HTTP request failed: {}", e))))?;

        if !response.status().is_success() {
            let error_text: String = response.text().await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(Error::new(LlmError(format!("OpenRouter API error: {}", error_text))));
        }

        let result: OpenRouterResponse = response.json().await
            .map_err(|e| Error::new(LlmError(format!("Failed to parse response: {}", e))))?;
        
        Ok(result
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default())
    }

    pub async fn extract_report_info(
        &self,
        conversation: &str,
    ) -> Result<serde_json::Value, Error> {
        let prompt = format!(
            r#"Ekstrak informasi laporan dari percakapan berikut. Return ONLY valid JSON dengan struktur:
{{
  "title": "judul singkat laporan (max 100 char)",
  "description": "deskripsi lengkap",
  "location_text": "lokasi yang disebutkan",
  "category": "kategori yang sesuai",
  "incident_date": "tanggal kejadian ISO format jika disebutkan, null jika tidak",
  "urgency": "low|medium|high"
}}

Percakapan:
{}
"#,
            conversation
        );

        let response = self
            .chat(
                vec![LlmMessage {
                    role: "user".to_string(),
                    content: prompt,
                }],
                None,
            )
            .await?;

        let json_str = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        serde_json::from_str(json_str)
            .map_err(|e| Error::new(LlmError(format!("Failed to parse JSON: {}", e))))
    }

    pub async fn check_completeness(
        &self,
        report_data: &serde_json::Value,
    ) -> Result<CompletenessResult, Error> {
        let prompt = format!(
            r#"Periksa kelengkapan laporan berikut dan beri skor 0-1. Return ONLY valid JSON:
{{
  "is_complete": true/false,
  "completeness_score": 0.0-1.0,
  "missing_fields": ["field1", "field2"],
  "suggestions": ["saran perbaikan"]
}}

Required fields untuk laporan lengkap:
- Deskripsi jelas masalah (minimal 20 karakter)
- Lokasi spesifik (nama jalan, kelurahan, atau koordinat)
- Waktu/tanggal kejadian (minimal perkiraan)
- Kategori masalah

Laporan:
{}
"#,
            serde_json::to_string_pretty(report_data)
                .map_err(|e| Error::new(LlmError(format!("Failed to serialize: {}", e))))?
        );

        let response = self
            .chat(
                vec![LlmMessage {
                    role: "user".to_string(),
                    content: prompt,
                }],
                None,
            )
            .await?;

        let json_str = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        serde_json::from_str(json_str)
            .map_err(|e| Error::new(LlmError(format!("Failed to parse JSON: {}", e))))
    }

    pub async fn extract_entities(&self, text: &str) -> Result<ExtractedEntities, Error> {
        let prompt = format!(
            r#"Ekstrak entitas dari teks laporan berikut. Return ONLY valid JSON:
{{
  "locations": ["lokasi1", "lokasi2"],
  "dates": ["tanggal1"],
  "organizations": ["instansi terkait"],
  "persons": ["nama orang jika ada"],
  "facilities": ["fasilitas yang disebutkan"]
}}

Teks: {}
"#,
            text
        );

        let response = self
            .chat(
                vec![LlmMessage {
                    role: "user".to_string(),
                    content: prompt,
                }],
                None,
            )
            .await?;

        let json_str = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();

        serde_json::from_str(json_str)
            .map_err(|e| Error::new(LlmError(format!("Failed to parse JSON: {}", e))))
    }

    pub async fn check_user_completion_intent(&self, message: &str) -> Result<bool, Error> {
        let prompt = format!(
            r#"Tentukan apakah pesan pengguna berikut mengindikasikan bahwa mereka ingin menyelesaikan/submit laporan mereka.
Return ONLY "true" atau "false" (tanpa quotes).

Contoh pesan yang menandakan selesai:
- "cukup sekian"
- "sudah selesai"
- "kirim saja"
- "simpan laporan"
- "itu saja"
- "selesai"

Pesan: {}
"#,
            message
        );

        let response = self
            .chat(
                vec![LlmMessage {
                    role: "user".to_string(),
                    content: prompt,
                }],
                None,
            )
            .await?;

        Ok(response.trim().to_lowercase() == "true")
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletenessResult {
    pub is_complete: bool,
    pub completeness_score: f64,
    pub missing_fields: Vec<String>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractedEntities {
    pub locations: Vec<String>,
    pub dates: Vec<String>,
    pub organizations: Vec<String>,
    pub persons: Vec<String>,
    pub facilities: Vec<String>,
}