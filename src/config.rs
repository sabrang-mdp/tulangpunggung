use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Config {
    pub db_host: String,
    pub db_port: u16,
    pub db_user: String,
    pub db_pass: String,
    pub db_name: String,
    pub host: String,
    pub port: u16,
    pub logto: LogtoConfig,
    pub openrouter: OpenRouterConfig,
    pub jwt_secret: String,
    pub clustering_interval_hours: u64,
    pub ner_processing_enabled: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LogtoConfig {
    pub endpoint: String,
    pub app_id: String,
    pub app_secret: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OpenRouterConfig {
    pub api_key: String,
    pub base_url: String,
}

#[derive(Debug)]
pub struct ConfigError(String);

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();
        
        Ok(Config {
            db_host: std::env::var("DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            db_port: std::env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string()).parse()?,
            db_user: std::env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
            db_pass: std::env::var("DB_PASS").map_err(|_| ConfigError("DB_PASS not set".into()))?,
            db_name: std::env::var("DB_NAME").map_err(|_| ConfigError("DB_NAME not set".into()))?,
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8000".to_string())
                .parse()
                .map_err(|_| ConfigError("Invalid PORT value".to_string()))?,
            logto: LogtoConfig {
                endpoint: std::env::var("LOGTO_ENDPOINT")
                    .map_err(|_| ConfigError("LOGTO_ENDPOINT not set".to_string()))?,
                app_id: std::env::var("LOGTO_APP_ID")
                    .map_err(|_| ConfigError("LOGTO_APP_ID not set".to_string()))?,
                app_secret: std::env::var("LOGTO_APP_SECRET")
                    .map_err(|_| ConfigError("LOGTO_APP_SECRET not set".to_string()))?,
            },
            openrouter: OpenRouterConfig {
                api_key: std::env::var("OPENROUTER_API_KEY")
                    .map_err(|_| ConfigError("OPENROUTER_API_KEY not set".to_string()))?,
                base_url: std::env::var("OPENROUTER_BASE_URL")
                    .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string()),
            },
            jwt_secret: std::env::var("JWT_SECRET")
                .map_err(|_| ConfigError("JWT_SECRET not set".to_string()))?,
            clustering_interval_hours: std::env::var("CLUSTERING_INTERVAL_HOURS")
                .unwrap_or_else(|_| "6".to_string())
                .parse()
                .map_err(|_| ConfigError("Invalid CLUSTERING_INTERVAL_HOURS value".to_string()))?,
            ner_processing_enabled: std::env::var("NER_PROCESSING_ENABLED")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .map_err(|_| ConfigError("Invalid NER_PROCESSING_ENABLED value".to_string()))?,
        })
    }

    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        Self::load()
    }

    pub fn build_rwf_db_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.db_user,
            urlencoding::encode(&self.db_pass), // Only encode for the URL string
            self.db_host,
            self.db_port,
            self.db_name
        )
    }
    
    pub fn server_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}