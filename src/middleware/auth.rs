use rwf::prelude::*;
use serde::{Deserialize, Serialize};
use jsonwebtoken::{decode, decode_header, DecodingKey, Validation, Algorithm};
use rwf::controller::Middleware;
use rwf::controller::Outcome;
use uuid::Uuid;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use once_cell::sync::Lazy;
use base64::{engine::general_purpose, Engine as _};

#[derive(Debug)]
struct AuthError(String);

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for AuthError {}

// Global storage for user context indexed by logto_user_id (sub claim)
static USER_CONTEXT: Lazy<Arc<RwLock<HashMap<String, UserContext>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

#[derive(Debug, Clone)]
struct UserContext {
    user_id: Uuid,
    user: crate::models::User,
}

#[derive(Debug, Clone)]
pub struct LogtoAuthMiddleware {
    logto_endpoint: String,
    app_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct Claims {
    sub: String,
    aud: String,
    exp: i64,
    iat: i64,
    iss: String,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JwksResponse {
    keys: Vec<Jwk>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Jwk {
    kid: String,
    kty: String,
    n: String,
    e: String,
}

impl LogtoAuthMiddleware {
    pub fn new(config: crate::config::LogtoConfig) -> Self {
        Self {
            logto_endpoint: config.endpoint,
            app_id: config.app_id,
        }
    }

    async fn get_jwks(&self) -> Result<JwksResponse, Error> {
        let jwks_url = format!("{}/oidc/jwks", self.logto_endpoint);
        let client = reqwest::Client::new();
        
        let response = client
            .get(&jwks_url)
            .send()
            .await
            .map_err(|e| Error::new(AuthError(format!("Failed to fetch JWKS: {}", e))))?;

        response
            .json()
            .await
            .map_err(|e| Error::new(AuthError(format!("Failed to parse JWKS: {}", e))))
    }

    async fn verify_token(&self, token: &str) -> Result<Claims, Error> {
        let header = decode_header(token)
            .map_err(|e| Error::new(AuthError(format!("Invalid token header: {}", e))))?;

        let kid = header.kid
            .ok_or_else(|| Error::new(AuthError("Missing kid in token".to_string())))?;

        let jwks = self.get_jwks().await?;
        
        let jwk = jwks
            .keys
            .iter()
            .find(|k| k.kid == kid)
            .ok_or_else(|| Error::new(AuthError("Key not found".to_string())))?;

        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.app_id]);
        validation.set_issuer(&[&format!("{}/oidc", self.logto_endpoint)]);

        let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
            .map_err(|e| Error::new(AuthError(format!("Invalid key: {}", e))))?;

        let token_data = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|e| Error::new(AuthError(format!("Token validation failed: {}", e))))?;

        Ok(token_data.claims)
    }

    async fn get_or_create_user(&self, claims: &Claims, pool: &sqlx::PgPool) -> Result<crate::models::User, Error> {
        let user = sqlx::query_as::<_, crate::models::User>(
            "SELECT * FROM users WHERE logto_user_id = $1"
        )
        .bind(&claims.sub)
        .fetch_optional(pool)
        .await
        .map_err(|e| Error::new(AuthError(format!("Database error: {}", e))))?;

        if let Some(user) = user {
            return Ok(user);
        }

        let new_user = sqlx::query_as::<_, crate::models::User>(
            r#"
            INSERT INTO users (logto_user_id, email, username, full_name, role)
            VALUES ($1, $2, $3, $4, 'user')
            RETURNING *
            "#
        )
        .bind(&claims.sub)
        .bind(&claims.email)
        .bind(&claims.username)
        .bind(&claims.name)
        .fetch_one(pool)
        .await
        .map_err(|e| Error::new(AuthError(format!("Failed to create user: {}", e))))?;

        Ok(new_user)
    }
}

#[async_trait]
impl Middleware for LogtoAuthMiddleware {
    async fn handle_request(&self, request: Request) -> Result<Outcome, Error> {
        let path = request.path().path();
        if path.starts_with("/auth/") || path == "/health" {
            return Ok(Outcome::Forward(request));
        }

        let auth_header = request
            .headers()
            .get("authorization")
            .or_else(|| request.headers().get("Authorization"))
            .ok_or_else(|| Error::new(AuthError("Missing authorization header".to_string())))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| Error::new(AuthError("Invalid authorization format".to_string())))?;

        let claims = self.verify_token(token).await?;

        let pool = crate::db::get_pool();
        let user = self.get_or_create_user(&claims, pool).await?;

        // Store user context indexed by logto_user_id
        {
            let mut context = USER_CONTEXT.write().unwrap();
            context.insert(claims.sub.clone(), UserContext {
                user_id: user.id,
                user: user.clone(),
            });
        }

        Ok(Outcome::Forward(request))
    }
}

pub trait RequestUserExt {
    fn user_id(&self) -> Result<Uuid, Error>;
    fn get_user(&self) -> Result<crate::models::User, Error>;
    fn require_role(&self, required_role: &str) -> Result<(), Error>;
}

impl RequestUserExt for Request {
    fn user_id(&self) -> Result<Uuid, Error> {
        // Extract token and get claims again
        let auth_header = self
            .headers()
            .get("authorization")
            .or_else(|| self.headers().get("Authorization"))
            .ok_or_else(|| Error::new(AuthError("Missing authorization header".to_string())))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| Error::new(AuthError("Invalid authorization format".to_string())))?;

        // Decode token without validation (we already validated in middleware)
        let token_data = decode::<Claims>(
            token, 
            &DecodingKey::from_secret(&[]), 
            &Validation::default()
        );
        
        // If decoding fails, try to extract sub from token manually
        let sub = if let Ok(data) = token_data {
            data.claims.sub
        } else {
            // Fallback: decode JWT payload manually
            let parts: Vec<&str> = token.split('.').collect();
            if parts.len() != 3 {
                return Err(Error::new(AuthError("Invalid token format".to_string())));
            }
            
            let payload = general_purpose::URL_SAFE_NO_PAD
                .decode(parts[1])
                .map_err(|_| Error::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid base64")))?;
            
            let claims: Claims = serde_json::from_slice(&payload)
                .map_err(|_| Error::new(AuthError("Failed to parse claims".to_string())))?;
            
            claims.sub
        };

        let context = USER_CONTEXT.read().unwrap();
        let user_context = context
            .get(&sub)
            .ok_or_else(|| Error::new(AuthError("User context not found".to_string())))?;

        Ok(user_context.user_id)
    }

    fn get_user(&self) -> Result<crate::models::User, Error> {
        // Extract token and get claims again
        let auth_header = self
            .headers()
            .get("authorization")
            .or_else(|| self.headers().get("Authorization"))
            .ok_or_else(|| Error::new(AuthError("Missing authorization header".to_string())))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| Error::new(AuthError("Invalid authorization format".to_string())))?;

        // Decode JWT payload manually
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(Error::new(AuthError("Invalid token format".to_string())));
        }
        
        let payload = general_purpose::URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|_| Error::new(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid base64")))?;
        
        let claims: Claims = serde_json::from_slice(&payload)
            .map_err(|_| Error::new(AuthError("Failed to parse claims".to_string())))?;

        let context = USER_CONTEXT.read().unwrap();
        let user_context = context
            .get(&claims.sub)
            .ok_or_else(|| Error::new(AuthError("User context not found".to_string())))?;

        Ok(user_context.user.clone())
    }

    fn require_role(&self, required_role: &str) -> Result<(), Error> {
        let user = self.get_user()?;
        
        let roles_hierarchy = vec!["user", "moderator", "admin"];
        let user_role_index = roles_hierarchy
            .iter()
            .position(|r| r == &user.role)
            .ok_or_else(|| Error::new(AuthError("Invalid user role".to_string())))?;

        let required_role_index = roles_hierarchy
            .iter()
            .position(|r| r == &required_role)
            .ok_or_else(|| Error::new(AuthError("Invalid required role".to_string())))?;

        if user_role_index < required_role_index {
            return Err(Error::new(AuthError("Insufficient permissions".to_string())));
        }

        Ok(())
    }
}