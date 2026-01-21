use rwf::prelude::*;
use uuid::Uuid;
use crate::models::*;

#[derive(Debug)]
struct ChatError(String);

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ChatError {}

#[derive(Default)]
pub struct ChatSessionsController;

#[async_trait]
impl Controller for ChatSessionsController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        use crate::middleware::auth::RequestUserExt;
        
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        let pool = crate::db::get_pool();
        
        let sessions = sqlx::query_as::<_, ChatSession>(
            "SELECT * FROM chat_sessions WHERE user_id = $1 ORDER BY last_message_at DESC"
        )
        .bind(user_id)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::new(ChatError(format!("Database error: {}", e))))?;
        
        Ok(Response::new()
            .json(&sessions)
            .map_err(|e| Error::new(ChatError(format!("JSON error: {}", e))))?)
    }
}

#[derive(Default)]
pub struct ChatSessionController;

#[async_trait]
impl Controller for ChatSessionController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        use crate::middleware::auth::RequestUserExt;
        
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        
        // request.parameter returns Result<Option<String>, Error>
        let session_id_str = match request.parameter::<String>("id")? {
            Some(id) => id,
            None => return Err(Error::new(ChatError("Missing session id".to_string()))),
        };
        
        let session_id = Uuid::parse_str(&session_id_str)
            .map_err(|_| Error::new(ChatError("Invalid session id".to_string())))?;
        
        let pool = crate::db::get_pool();
        
        let session = sqlx::query_as::<_, ChatSession>(
            "SELECT * FROM chat_sessions WHERE id = $1 AND user_id = $2"
        )
        .bind(session_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::new(ChatError(format!("Database error: {}", e))))?
        .ok_or_else(|| Error::new(ChatError("Session not found".to_string())))?;
        
        Ok(Response::new()
            .json(&session)
            .map_err(|e| Error::new(ChatError(format!("JSON error: {}", e))))?)
    }
}

#[derive(Default)]
pub struct ChatMessagesController;

#[async_trait]
impl Controller for ChatMessagesController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        use crate::middleware::auth::RequestUserExt;
        
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        
        // request.parameter returns Result<Option<String>, Error>
        let session_id_str = match request.parameter::<String>("id")? {
            Some(id) => id,
            None => return Err(Error::new(ChatError("Missing session id".to_string()))),
        };
        
        let session_id = Uuid::parse_str(&session_id_str)
            .map_err(|_| Error::new(ChatError("Invalid session id".to_string())))?;
        
        let pool = crate::db::get_pool();
        
        let session = sqlx::query_as::<_, ChatSession>(
            "SELECT * FROM chat_sessions WHERE id = $1 AND user_id = $2"
        )
        .bind(session_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::new(ChatError(format!("Database error: {}", e))))?
        .ok_or_else(|| Error::new(ChatError("Session not found".to_string())))?;
        
        let messages = sqlx::query_as::<_, ChatMessage>(
            "SELECT * FROM chat_messages WHERE session_id = $1 ORDER BY created_at ASC"
        )
        .bind(session.id)
        .fetch_all(pool)
        .await
        .map_err(|e| Error::new(ChatError(format!("Database error: {}", e))))?;
        
        Ok(Response::new()
            .json(&messages)
            .map_err(|e| Error::new(ChatError(format!("JSON error: {}", e))))?)
    }
}