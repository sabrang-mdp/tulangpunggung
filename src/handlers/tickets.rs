use rwf::prelude::*;
use uuid::Uuid;
use serde::Deserialize;
use crate::models::*;
use crate::middleware::auth::RequestUserExt;

#[derive(Default)]
pub struct TicketsListController;

#[async_trait]
impl Controller for TicketsListController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        let pool = crate::db::get_pool();
        
        let query = request.query();
        let status = query.get::<String>("status");
        let priority = query.get::<String>("priority");
        let assigned_to = query.get::<String>("assigned_to")
            .and_then(|s| Uuid::parse_str(&s).ok());
        let limit: i64 = query.get::<i64>("limit").unwrap_or(50).min(100);
        let offset: i64 = query.get::<i64>("offset").unwrap_or(0);
        
        let mut sql = String::from("SELECT t.* FROM tickets t WHERE t.user_id = $1");
        
        let mut bind_index = 2;
        if status.is_some() {
            sql.push_str(&format!(" AND t.status = ${}", bind_index));
            bind_index += 1;
        }
        if priority.is_some() {
            sql.push_str(&format!(" AND t.priority = ${}", bind_index));
            bind_index += 1;
        }
        if assigned_to.is_some() {
            sql.push_str(&format!(" AND t.assigned_to = ${}", bind_index));
            bind_index += 1;
        }
        
        sql.push_str(&format!(" ORDER BY t.created_at DESC LIMIT ${} OFFSET ${}", bind_index, bind_index + 1));
        
        let mut query_builder = sqlx::query_as::<_, Ticket>(&sql).bind(user_id);
        
        if let Some(ref status) = status {
            query_builder = query_builder.bind(status);
        }
        if let Some(ref priority) = priority {
            query_builder = query_builder.bind(priority);
        }
        if let Some(assigned_to) = assigned_to {
            query_builder = query_builder.bind(assigned_to);
        }
        
        let tickets = query_builder
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(Error::new)?;
        
        Response::new().json(&tickets).map_err(Error::new)
    }
}

#[derive(Default)]
pub struct TicketController;

#[async_trait]
impl Controller for TicketController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        let pool = crate::db::get_pool();
        
        let id_str = request.parameter::<String>("id")?.unwrap_or_default();
        let id = Uuid::parse_str(&id_str).map_err(Error::new)?;
        
        let ticket = sqlx::query_as::<_, Ticket>(
            "SELECT * FROM tickets WHERE id = $1 AND user_id = $2"
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(Error::new)?
        .ok_or_else(|| Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Ticket not found")))?;
        
        let report = sqlx::query_as::<_, Report>(
            "SELECT * FROM reports WHERE id = $1"
        )
        .bind(ticket.report_id)
        .fetch_one(pool)
        .await
        .map_err(Error::new)?;
        
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE id = $1"
        )
        .bind(ticket.user_id)
        .fetch_one(pool)
        .await
        .map_err(Error::new)?;
        
        let assigned_user = if let Some(assigned_id) = ticket.assigned_to {
            sqlx::query_as::<_, User>(
                "SELECT * FROM users WHERE id = $1"
            )
            .bind(assigned_id)
            .fetch_optional(pool)
            .await
            .map_err(Error::new)?
        } else {
            None
        };
        
        let ticket_with_details = TicketWithDetails {
            ticket,
            report,
            user,
            assigned_user,
        };
        
        Response::new().json(&ticket_with_details).map_err(Error::new)
    }
}

#[derive(Default)]
pub struct TicketCommentsController;

#[async_trait]
impl Controller for TicketCommentsController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        let pool = crate::db::get_pool();
        
        let id_str = request.parameter::<String>("id")?.unwrap_or_default();
        let id = Uuid::parse_str(&id_str).map_err(Error::new)?;
        let req: AddCommentRequest = request.json().map_err(Error::new)?;
        
        let ticket = sqlx::query_as::<_, Ticket>(
            "SELECT * FROM tickets WHERE id = $1 AND user_id = $2"
        )
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(Error::new)?
        .ok_or_else(|| Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Ticket not found")))?;
        
        let comment = sqlx::query_as::<_, TicketComment>(
            "INSERT INTO ticket_comments (ticket_id, user_id, comment, is_internal)
             VALUES ($1, $2, $3, $4)
             RETURNING *"
        )
        .bind(ticket.id)
        .bind(user_id)
        .bind(req.comment)
        .bind(req.is_internal.unwrap_or(false))
        .fetch_one(pool)
        .await
        .map_err(Error::new)?;
        
        Response::new().json(&comment).map_err(Error::new)
    }
}

#[derive(Default)]
pub struct TicketStatusController;

#[async_trait]
impl Controller for TicketStatusController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        let pool = crate::db::get_pool();
        
        let id_str = request.parameter::<String>("id")?.unwrap_or_default();
        let id = Uuid::parse_str(&id_str).map_err(Error::new)?;
        let req: UpdateStatusRequest = request.json().map_err(Error::new)?;
        
        let is_resolved = req.status == "resolved" || req.status == "closed";
        
        let ticket = sqlx::query_as::<_, Ticket>(
            "UPDATE tickets SET
                status = $1,
                resolution = COALESCE($2, resolution),
                resolved_at = CASE WHEN $3 THEN NOW() ELSE resolved_at END,
                resolved_by = CASE WHEN $3 THEN $4 ELSE resolved_by END,
                updated_at = NOW()
             WHERE id = $5 AND user_id = $6
             RETURNING *"
        )
        .bind(req.status)
        .bind(req.resolution)
        .bind(is_resolved)
        .bind(user_id)
        .bind(id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(Error::new)?
        .ok_or_else(|| Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Ticket not found")))?;
        
        Response::new().json(&ticket).map_err(Error::new)
    }
}

#[derive(Deserialize)]
pub struct UpdateStatusRequest {
    pub status: String,
    pub resolution: Option<String>,
}