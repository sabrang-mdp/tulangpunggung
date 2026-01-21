use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// User Model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: Uuid,
    pub logto_user_id: String,
    pub email: Option<String>,
    pub username: Option<String>,
    pub full_name: Option<String>,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_active: bool,
}

// Category Model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Category {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<Uuid>,
    pub is_active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoryRequest {
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
}

// Chat Session Model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChatSession {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_message_at: DateTime<Utc>,
}

// Chat Message Model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ChatMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessageRequest {
    pub content: String,
}

// Report Model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Report {
    pub id: Uuid,
    pub session_id: Uuid,
    pub user_id: Uuid,
    pub category_id: Option<Uuid>,
    pub title: String,
    pub description: String,
    pub location_text: Option<String>,
    pub latitude: Option<rust_decimal::Decimal>,
    pub longitude: Option<rust_decimal::Decimal>,
    pub address: Option<String>,
    pub incident_date: Option<DateTime<Utc>>,
    pub reported_date: DateTime<Utc>,
    pub status: String,
    pub is_complete: bool,
    pub completeness_score: rust_decimal::Decimal,
    pub missing_fields: serde_json::Value,
    pub entities: serde_json::Value,
    pub cluster_id: Option<Uuid>,
    pub attachments: serde_json::Value,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateReportRequest {
    pub session_id: Uuid,
    pub title: String,
    pub description: String,
    pub category_id: Option<Uuid>,
    pub location_text: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub incident_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, FromRow)] 
pub struct ReportWithCategory {
    #[sqlx(flatten)]
    pub report: Report,
    pub category: Option<serde_json::Value>, 
}

// Report Cluster Model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ReportCluster {
    pub id: Uuid,
    pub name: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<Uuid>,
    pub centroid: serde_json::Value,
    pub report_count: i32,
    pub center_latitude: Option<rust_decimal::Decimal>,
    pub center_longitude: Option<rust_decimal::Decimal>,
    pub radius_meters: Option<rust_decimal::Decimal>,
    pub earliest_incident: Option<DateTime<Utc>>,
    pub latest_incident: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Ticket Model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Ticket {
    pub id: Uuid,
    pub ticket_number: String,
    pub report_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub priority: String,
    pub assigned_to: Option<Uuid>,
    pub assigned_at: Option<DateTime<Utc>>,
    pub resolution: Option<String>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolved_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TicketWithDetails {
    #[serde(flatten)]
    pub ticket: Ticket,
    pub report: Report,
    pub user: User,
    pub assigned_user: Option<User>,
}

// Ticket Comment Model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct TicketComment {
    pub id: Uuid,
    pub ticket_id: Uuid,
    pub user_id: Uuid,
    pub comment: String,
    pub is_internal: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AddCommentRequest {
    pub comment: String,
    pub is_internal: Option<bool>,
}

// API Key Model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ApiKey {
    pub id: Uuid,
    pub name: String,
    pub provider: String,
    #[serde(skip_serializing)]
    pub api_key: String,
    pub base_url: Option<String>,
    pub is_active: bool,
    pub usage_count: i64,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
}

// System Prompt Model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct SystemPrompt {
    pub id: Uuid,
    pub name: String,
    pub prompt_type: String,
    pub prompt_text: String,
    pub variables: serde_json::Value,
    pub is_active: bool,
    pub version: i32,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePromptRequest {
    pub name: String,
    pub prompt_type: String,
    pub prompt_text: String,
    pub variables: Option<serde_json::Value>,
}

// Background Job Model
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BackgroundJob {
    pub id: Uuid,
    pub job_type: String,
    pub status: String,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

// Dashboard Models
#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub total_reports: i64,
    pub active_reports: i64,
    pub resolved_reports: i64,
    pub total_users: i64,
    pub reports_this_week: i64,
    pub reports_this_month: i64,
    pub average_resolution_time_hours: Option<f64>,
    pub top_categories: Vec<CategoryStats>,
}

#[derive(Debug, Serialize, FromRow)]  // Added FromRow here
pub struct CategoryStats {
    pub category_id: Uuid,
    pub category_name: String,
    pub report_count: i64,
    pub percentage: f64,
}

#[derive(Debug, Serialize, FromRow)]  // Added FromRow here
pub struct TrendData {
    pub date: String,
    pub count: i64,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, FromRow)]  // Added FromRow here
pub struct HeatmapPoint {
    pub latitude: f64,
    pub longitude: f64,
    pub intensity: i32,
    pub reports: Vec<Uuid>,
}

// WebSocket Messages
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    UserMessage {
        content: String,
    },
    AssistantMessage {
        content: String,
    },
    SystemMessage {
        content: String,
    },
    CompletenessCheck {
        is_complete: bool,
        score: f64,
        missing_fields: Vec<String>,
        suggestions: Vec<String>,
    },
    ReportCreated {
        report_id: Uuid,
        ticket_number: String,
    },
    Error {
        message: String,
    },
}