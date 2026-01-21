use rwf::prelude::*;
use rwf::http::websocket::Message;
use rwf::controller::WebsocketController;
use uuid::Uuid;
use crate::models::*;

#[derive(Default, macros::WebsocketController)]
pub struct ChatWebSocketController;

#[async_trait]
impl WebsocketController for ChatWebSocketController {
    async fn client_connected(&self, session_id: &SessionId) -> Result<(), Error> {
        tracing::info!("Client {:?} connected to chat", session_id);
        Ok(())
    }
    
    async fn client_message(&self, session_id: &SessionId, message: Message) -> Result<(), Error> {
        let pool = crate::db::get_pool();
        
        let message_text = match message {
            Message::Text(text) => text,
            _ => return Ok(()),
        };
        
        let chat_session_id = match get_or_create_session(pool, session_id).await {
            Ok(id) => id,
            Err(e) => {
                tracing::error!("Failed to get/create session: {:?}", e);
                return Ok(());
            }
        };
        
        if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&message_text) {
            match ws_msg {
                WsMessage::UserMessage { content } => {
                    if let Err(e) = handle_user_message(pool, chat_session_id, session_id, &content).await {
                        tracing::error!("Error handling user message: {:?}", e);
                        let error_msg = WsMessage::Error {
                            message: "Failed to process message".to_string(),
                        };
                        if let Ok(json) = serde_json::to_string(&error_msg) {
                            let _ = Comms::websocket(session_id).send(json);
                        }
                    }
                }
                _ => {}
            }
        }
        
        Ok(())
    }
}

async fn get_or_create_session(
    pool: &sqlx::PgPool,
    client: &SessionId,
) -> Result<Uuid, sqlx::Error> {
    let client_str = format!("{:?}", client);
    
    let existing: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM chat_sessions 
         WHERE metadata->>'client_id' = $1 
         AND status = 'active'
         ORDER BY created_at DESC
         LIMIT 1"
    )
    .bind(&client_str)
    .fetch_optional(pool)
    .await?;
    
    if let Some((session_id,)) = existing {
        return Ok(session_id);
    }
    
    let session_id: Uuid = sqlx::query_scalar(
        "INSERT INTO chat_sessions (user_id, status, metadata)
         VALUES ('00000000-0000-0000-0000-000000000000', 'active', $1)
         RETURNING id"
    )
    .bind(serde_json::json!({ "client_id": client_str }))
    .fetch_one(pool)
    .await?;
    
    Ok(session_id)
}

async fn handle_user_message(
    pool: &sqlx::PgPool,
    session_id: Uuid,
    client: &SessionId,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query(
        "INSERT INTO chat_messages (session_id, role, content)
         VALUES ($1, 'user', $2)"
    )
    .bind(session_id)
    .bind(content)
    .execute(pool)
    .await?;
    
    sqlx::query(
        "UPDATE chat_sessions 
         SET last_message_at = NOW() 
         WHERE id = $1"
    )
    .bind(session_id)
    .execute(pool)
    .await?;
    
    let messages: Vec<ChatMessage> = sqlx::query_as(
        "SELECT * FROM chat_messages 
         WHERE session_id = $1 
         ORDER BY created_at ASC"
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;
    
    let system_prompt = sqlx::query_as::<_, SystemPrompt>(
        "SELECT * FROM system_prompts 
         WHERE prompt_type = 'chat_assistant' 
         AND is_active = true 
         ORDER BY version DESC 
         LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;
    
    let prompt_text = system_prompt
        .map(|p| p.prompt_text)
        .unwrap_or_else(|| "Anda adalah asisten yang membantu.".to_string());
    
    let api_key = sqlx::query_as::<_, ApiKey>(
        "SELECT * FROM api_keys 
         WHERE is_active = true 
         ORDER BY created_at DESC 
         LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;
    
    if api_key.is_none() {
        let error_msg = WsMessage::Error {
            message: "No active API key configured".to_string(),
        };
        if let Ok(json) = serde_json::to_string(&error_msg) {
            Comms::websocket(client).send(json)?;
        }
        return Ok(());
    }
    
    let api_key = api_key.unwrap();
    
    let response = call_llm_api(&api_key, &prompt_text, &messages, content).await?;
    
    sqlx::query(
        "INSERT INTO chat_messages (session_id, role, content)
         VALUES ($1, 'assistant', $2)"
    )
    .bind(session_id)
    .bind(&response)
    .execute(pool)
    .await?;
    
    sqlx::query(
        "UPDATE api_keys SET usage_count = usage_count + 1, last_used_at = NOW() WHERE id = $1"
    )
    .bind(api_key.id)
    .execute(pool)
    .await?;
    
    let assistant_msg = WsMessage::AssistantMessage {
        content: response,
    };
    if let Ok(json) = serde_json::to_string(&assistant_msg) {
        Comms::websocket(client).send(json)?;
    }
    
    check_completeness(pool, session_id, client).await?;
    
    Ok(())
}

async fn call_llm_api(
    api_key: &ApiKey,
    system_prompt: &str,
    history: &[ChatMessage],
    user_message: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    
    let base_url = api_key.base_url.as_deref().unwrap_or("https://openrouter.ai/api/v1");
    
    let mut messages_payload = vec![
        serde_json::json!({
            "role": "system",
            "content": system_prompt
        })
    ];
    
    for msg in history {
        messages_payload.push(serde_json::json!({
            "role": msg.role,
            "content": msg.content
        }));
    }
    
    messages_payload.push(serde_json::json!({
        "role": "user",
        "content": user_message
    }));
    
    let response = client
        .post(format!("{}/chat/completions", base_url))
        .header("Authorization", format!("Bearer {}", api_key.api_key))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "openai/gpt-3.5-turbo",
            "messages": messages_payload,
            "temperature": 0.7
        }))
        .send()
        .await?;
    
    let result: serde_json::Value = response.json().await?;
    
    let content = result["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("Error processing response")
        .to_string();
    
    Ok(content)
}

async fn check_completeness(
    pool: &sqlx::PgPool,
    session_id: Uuid,
    client: &SessionId,
) -> Result<(), Box<dyn std::error::Error>> {
    let messages: Vec<ChatMessage> = sqlx::query_as(
        "SELECT * FROM chat_messages WHERE session_id = $1 ORDER BY created_at ASC"
    )
    .bind(session_id)
    .fetch_all(pool)
    .await?;
    
    let conversation = messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");
    
    let report_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM reports WHERE session_id = $1"
    )
    .bind(session_id)
    .fetch_one(pool)
    .await?;
    
    if report_count == 0 && conversation.len() > 100 {
        if conversation.contains("jalan") || conversation.contains("rusak") {
            let title = "Laporan dari percakapan".to_string();
            let description = conversation.lines().take(5).collect::<Vec<_>>().join(" ");
            
            let report_id: Uuid = sqlx::query_scalar(
                "INSERT INTO reports (
                    session_id, user_id, title, description, 
                    status, is_complete, completeness_score
                ) VALUES ($1, '00000000-0000-0000-0000-000000000000', $2, $3, 'draft', false, 0.5)
                RETURNING id"
            )
            .bind(session_id)
            .bind(title)
            .bind(description)
            .fetch_one(pool)
            .await?;
            
            let ticket_number = format!("TKT-{}", &report_id.to_string()[..8].to_uppercase());
            
            let completeness_msg = WsMessage::CompletenessCheck {
                is_complete: false,
                score: 0.5,
                missing_fields: vec!["location".to_string(), "incident_date".to_string()],
                suggestions: vec!["Mohon sebutkan lokasi spesifik".to_string()],
            };
            if let Ok(json) = serde_json::to_string(&completeness_msg) {
                Comms::websocket(client).send(json)?;
            }
            
            let created_msg = WsMessage::ReportCreated {
                report_id,
                ticket_number,
            };
            if let Ok(json) = serde_json::to_string(&created_msg) {
                Comms::websocket(client).send(json)?;
            }
        }
    }
    
    Ok(())
}