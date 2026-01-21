use rwf::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug)]
struct AuthHandlerError(String);

impl std::fmt::Display for AuthHandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AuthHandlerError {}

#[derive(Default)]
pub struct CallbackController;

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: String,
    state: Option<String>,
}

#[derive(Debug, Serialize)]
struct TokenRequest {
    grant_type: String,
    code: String,
    redirect_uri: String,
    client_id: String,
    client_secret: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    id_token: String,
    token_type: String,
    expires_in: i64,
}

#[async_trait]
impl Controller for CallbackController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        let config = crate::config::Config::load()
            .map_err(|e| Error::new(AuthHandlerError(format!("Config error: {}", e))))?;

        let code = request.query().get::<String>("code")
            .ok_or_else(|| Error::new(AuthHandlerError("Missing code parameter".to_string())))?;

        let client = reqwest::Client::new();
        let token_url = format!("{}/oidc/token", config.logto.endpoint);

        let token_request = TokenRequest {
            grant_type: "authorization_code".to_string(),
            code,
            redirect_uri: format!("{}/auth/callback", config.host),
            client_id: config.logto.app_id.clone(),
            client_secret: config.logto.app_secret.clone(),
        };

        let response = client
            .post(&token_url)
            .json(&token_request)
            .send()
            .await
            .map_err(|e| Error::new(AuthHandlerError(format!("HTTP request failed: {}", e))))?;

        if !response.status().is_success() {
            let error_text: String = response.text().await.unwrap_or_default();
            return Err(Error::new(AuthHandlerError(format!("Token exchange failed: {}", error_text))));
        }

        let token_response: TokenResponse = response
            .json()
            .await
            .map_err(|e| Error::new(AuthHandlerError(format!("Failed to parse response: {}", e))))?;

        let json_response = json!({
            "access_token": token_response.access_token,
            "id_token": token_response.id_token,
            "token_type": token_response.token_type,
            "expires_in": token_response.expires_in,
        });

        Ok(Response::new()
            .json(&json_response)
            .map_err(|e| Error::new(AuthHandlerError(format!("JSON error: {}", e))))?)
    }
}

#[derive(Default)]
pub struct MeController;

#[async_trait]
impl Controller for MeController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        use crate::middleware::auth::RequestUserExt;
        
        let user = request.get_user()?;

        Ok(Response::new()
            .json(&user)
            .map_err(|e| Error::new(AuthHandlerError(format!("JSON error: {}", e))))?)
    }
}

#[derive(Default)]
pub struct LogoutController;

#[async_trait]
impl Controller for LogoutController {
    async fn handle(&self, _request: &Request) -> Result<Response, Error> {
        let config = crate::config::Config::load()
            .map_err(|e| Error::new(AuthHandlerError(format!("Config error: {}", e))))?;

        let logout_url = format!(
            "{}/oidc/logout?post_logout_redirect_uri={}",
            config.logto.endpoint,
            urlencoding::encode(&format!("{}/", config.host))
        );

        let json_response = json!({
            "logout_url": logout_url,
        });

        Ok(Response::new()
            .json(&json_response)
            .map_err(|e| Error::new(AuthHandlerError(format!("JSON error: {}", e))))?)
    }
}