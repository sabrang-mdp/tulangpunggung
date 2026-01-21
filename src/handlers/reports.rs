use rwf::prelude::*;
use uuid::Uuid;
use crate::models::*;
use crate::middleware::auth::RequestUserExt;

#[derive(Default, macros::RestController)]
pub struct ReportsController;

#[async_trait]
impl RestController for ReportsController {
    type Resource = String;
    
    async fn list(&self, request: &Request) -> Result<Response, Error> {
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        let pool = crate::db::get_pool();
        
        let query = request.query();
        let status = query.get::<String>("status");
        let category_id = query.get::<String>("category_id")
            .and_then(|s| Uuid::parse_str(&s).ok());
        
        let limit = query.get::<i64>("limit").unwrap_or(50).min(100);
        let offset = query.get::<i64>("offset").unwrap_or(0);
        
        let mut sql = String::from(
            "SELECT r.*, row_to_json(c.*) as category FROM reports r LEFT JOIN categories c ON r.category_id = c.id WHERE r.user_id = $1"
        );
        
        let mut bind_index = 2;
        if status.is_some() {
            sql.push_str(&format!(" AND r.status = ${}", bind_index));
            bind_index += 1;
        }
        if category_id.is_some() {
            sql.push_str(&format!(" AND r.category_id = ${}", bind_index));
            bind_index += 1;
        }
        
        sql.push_str(&format!(" ORDER BY r.created_at DESC LIMIT ${} OFFSET ${}", bind_index, bind_index + 1));
        
        let mut query_obj = sqlx::query_as::<sqlx::Postgres, ReportWithCategory>(&sql).bind(user_id);
        
        if let Some(ref s) = status { query_obj = query_obj.bind(s); }
        if let Some(c) = category_id { query_obj = query_obj.bind(c); }
        
        let reports = query_obj
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(Error::new)?;
        
        Response::new().json(&reports).map_err(Error::new)
    }
    
    async fn get(&self, request: &Request, id: &String) -> Result<Response, Error> {
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        let pool = crate::db::get_pool();
        let report_id = Uuid::parse_str(id).map_err(Error::new)?;
        
        let report = sqlx::query_as::<sqlx::Postgres, ReportWithCategory>(
            "SELECT r.*, row_to_json(c.*) as category FROM reports r LEFT JOIN categories c ON r.category_id = c.id WHERE r.id = $1 AND r.user_id = $2"
        )
        .bind(report_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(Error::new)?
        .ok_or_else(|| Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Report not found")))?;
        
        Response::new().json(&report).map_err(Error::new)
    }
    
    async fn create(&self, request: &Request) -> Result<Response, Error> {
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        let pool = crate::db::get_pool();
        let req: CreateReportRequest = request.json().map_err(Error::new)?;
        
        let report = sqlx::query_as::<sqlx::Postgres, Report>(
            "INSERT INTO reports (session_id, user_id, category_id, title, description, location_text, latitude, longitude, incident_date, status, is_complete, completeness_score) 
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'submitted', false, 0.0) RETURNING *"
        )
        .bind(req.session_id).bind(user_id).bind(req.category_id)
        .bind(req.title).bind(req.description).bind(req.location_text)
        .bind(req.latitude).bind(req.longitude).bind(req.incident_date)
        .fetch_one(pool).await.map_err(Error::new)?;
        
        let tkt = format!("TKT-{}", &report.id.to_string()[..8].to_uppercase());
        sqlx::query("INSERT INTO tickets (ticket_number, report_id, user_id, status, priority) VALUES ($1, $2, $3, 'open', 'medium')")
            .bind(&tkt).bind(report.id).bind(user_id).execute(pool).await.map_err(Error::new)?;
        
        Response::new().json(&report).map_err(Error::new)
    }

    async fn update(&self, request: &Request, id: &String) -> Result<Response, Error> {
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        let pool = crate::db::get_pool();
        let report_id = Uuid::parse_str(id).map_err(Error::new)?;
        let req: UpdateReportRequest = request.json().map_err(Error::new)?;
        
        let report = sqlx::query_as::<sqlx::Postgres, Report>(
            "UPDATE reports SET title = COALESCE($1, title), description = COALESCE($2, description), updated_at = NOW() WHERE id = $3 AND user_id = $4 RETURNING *"
        )
        .bind(req.title).bind(req.description).bind(report_id).bind(user_id)
        .fetch_one(pool).await.map_err(Error::new)?;
        
        Response::new().json(&report).map_err(Error::new)
    }
}

#[derive(serde::Deserialize)]
pub struct UpdateReportRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub category_id: Option<Uuid>,
}

#[derive(Default)]
pub struct ReportCompleteController;

#[async_trait]
impl Controller for ReportCompleteController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        let pool = crate::db::get_pool();
        let id_str = request.parameter::<String>("id")?.unwrap_or_default();
        let id = Uuid::parse_str(&id_str).map_err(Error::new)?;
        
        let report = sqlx::query_as::<sqlx::Postgres, Report>(
            "UPDATE reports SET is_complete = true, updated_at = NOW() WHERE id = $1 AND user_id = $2 RETURNING *"
        )
        .bind(id).bind(user_id).fetch_one(pool).await.map_err(Error::new)?;
        
        Response::new().json(&report).map_err(Error::new)
    }
}