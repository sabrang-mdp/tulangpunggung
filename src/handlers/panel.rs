use rwf::prelude::*;
use uuid::Uuid;
use serde::Deserialize;
use crate::models::*;
use crate::middleware::auth::RequestUserExt;

#[derive(Default)]
pub struct AdminUsersController;

#[async_trait]
impl Controller for AdminUsersController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        let query = request.query();
        
        let role = query.get::<String>("role");
        let is_active = query.get::<bool>("is_active");
        let limit: i64 = query.get::<i64>("limit").unwrap_or(50).min(100);
        let offset: i64 = query.get::<i64>("offset").unwrap_or(0);
        
        let mut sql = String::from("SELECT * FROM users WHERE 1=1");
        
        let mut bind_index = 1;
        if role.is_some() {
            sql.push_str(&format!(" AND role = ${}", bind_index));
            bind_index += 1;
        }
        if is_active.is_some() {
            sql.push_str(&format!(" AND is_active = ${}", bind_index));
            bind_index += 1;
        }
        
        sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ${} OFFSET ${}", bind_index, bind_index + 1));
        
        let mut query_builder = sqlx::query_as::<_, User>(&sql);
        
        if let Some(ref role) = role {
            query_builder = query_builder.bind(role);
        }
        if let Some(is_active) = is_active {
            query_builder = query_builder.bind(is_active);
        }
        
        let users = query_builder
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(Error::new)?;
        
        Response::new().json(&users).map_err(Error::new)
    }
}

#[derive(Default)]
pub struct AdminUserRoleController;

#[async_trait]
impl Controller for AdminUserRoleController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        
        let id_str = request.parameter::<String>("id")?.unwrap_or_default();
        let id = Uuid::parse_str(&id_str).map_err(Error::new)?;
        let req: UpdateUserRoleRequest = request.json().map_err(Error::new)?;
        
        let user = sqlx::query_as::<_, User>(
            "UPDATE users SET role = $1, updated_at = NOW() WHERE id = $2 RETURNING *"
        )
        .bind(req.role)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(Error::new)?
        .ok_or_else(|| Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "User not found")))?;
        
        Response::new().json(&user).map_err(Error::new)
    }
}

#[derive(Deserialize)]
pub struct UpdateUserRoleRequest {
    pub role: String,
}

#[derive(Default, macros::RestController)]
pub struct CategoriesController;

#[async_trait]
impl RestController for CategoriesController {
    type Resource = String;
    
    async fn list(&self, _request: &Request) -> Result<Response, Error> {
        let pool = crate::db::get_pool();
        
        let categories = sqlx::query_as::<_, Category>(
            "SELECT * FROM categories WHERE is_active = true ORDER BY name ASC"
        )
        .fetch_all(pool)
        .await
        .map_err(Error::new)?;
        
        Response::new().json(&categories).map_err(Error::new)
    }
    
    async fn get(&self, _request: &Request, id: &String) -> Result<Response, Error> {
        let pool = crate::db::get_pool();
        let category_id = Uuid::parse_str(id).map_err(Error::new)?;
        
        let category = sqlx::query_as::<_, Category>(
            "SELECT * FROM categories WHERE id = $1"
        )
        .bind(category_id)
        .fetch_optional(pool)
        .await
        .map_err(Error::new)?
        .ok_or_else(|| Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Category not found")))?;
        
        Response::new().json(&category).map_err(Error::new)
    }
    
    async fn create(&self, request: &Request) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        let req: CreateCategoryRequest = request.json().map_err(Error::new)?;
        
        let category = sqlx::query_as::<_, Category>(
            "INSERT INTO categories (name, description, icon, color, created_by)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *"
        )
        .bind(req.name)
        .bind(req.description)
        .bind(req.icon)
        .bind(req.color)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(Error::new)?;
        
        Response::new().json(&category).map_err(Error::new)
    }
    
    async fn update(&self, request: &Request, id: &String) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        let req: UpdateCategoryRequest = request.json().map_err(Error::new)?;
        let category_id = Uuid::parse_str(id).map_err(Error::new)?;
        
        let category = sqlx::query_as::<_, Category>(
            "UPDATE categories SET
                name = COALESCE($1, name),
                description = COALESCE($2, description),
                icon = COALESCE($3, icon),
                color = COALESCE($4, color),
                is_active = COALESCE($5, is_active),
                updated_at = NOW()
             WHERE id = $6
             RETURNING *"
        )
        .bind(req.name)
        .bind(req.description)
        .bind(req.icon)
        .bind(req.color)
        .bind(req.is_active)
        .bind(category_id)
        .fetch_optional(pool)
        .await
        .map_err(Error::new)?
        .ok_or_else(|| Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Category not found")))?;
        
        Response::new().json(&category).map_err(Error::new)
    }
    
    async fn delete(&self, request: &Request, id: &String) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        let category_id = Uuid::parse_str(id).map_err(Error::new)?;
        
        let result = sqlx::query(
            "UPDATE categories SET is_active = false, updated_at = NOW() WHERE id = $1"
        )
        .bind(category_id)
        .execute(pool)
        .await
        .map_err(Error::new)?;
        
        if result.rows_affected() == 0 {
            return Err(Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Category not found")));
        }
        
        Ok(Response::new())
    }
}

#[derive(Deserialize)]
pub struct UpdateCategoryRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Default, macros::RestController)]
pub struct PromptsController;

#[async_trait]
impl RestController for PromptsController {
    type Resource = String;
    
    async fn list(&self, request: &Request) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        
        let prompts = sqlx::query_as::<_, SystemPrompt>(
            "SELECT * FROM system_prompts ORDER BY created_at DESC"
        )
        .fetch_all(pool)
        .await
        .map_err(Error::new)?;
        
        Response::new().json(&prompts).map_err(Error::new)
    }
    
    async fn get(&self, request: &Request, id: &String) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        let prompt_id = Uuid::parse_str(id).map_err(Error::new)?;
        
        let prompt = sqlx::query_as::<_, SystemPrompt>(
            "SELECT * FROM system_prompts WHERE id = $1"
        )
        .bind(prompt_id)
        .fetch_optional(pool)
        .await
        .map_err(Error::new)?
        .ok_or_else(|| Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Prompt not found")))?;
        
        Response::new().json(&prompt).map_err(Error::new)
    }
    
    async fn create(&self, request: &Request) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        let req: CreatePromptRequest = request.json().map_err(Error::new)?;
        
        let variables = req.variables.unwrap_or(serde_json::json!({}));
        
        let prompt = sqlx::query_as::<_, SystemPrompt>(
            "INSERT INTO system_prompts (name, prompt_type, prompt_text, variables, created_by)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *"
        )
        .bind(req.name)
        .bind(req.prompt_type)
        .bind(req.prompt_text)
        .bind(variables)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(Error::new)?;
        
        Response::new().json(&prompt).map_err(Error::new)
    }
    
    async fn update(&self, request: &Request, id: &String) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        let req: UpdatePromptRequest = request.json().map_err(Error::new)?;
        let prompt_id = Uuid::parse_str(id).map_err(Error::new)?;
        
        let existing = sqlx::query_as::<_, SystemPrompt>(
            "SELECT * FROM system_prompts WHERE id = $1"
        )
        .bind(prompt_id)
        .fetch_optional(pool)
        .await
        .map_err(Error::new)?
        .ok_or_else(|| Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Prompt not found")))?;
        
        let new_version = if req.prompt_text.is_some() {
            existing.version + 1
        } else {
            existing.version
        };
        
        let prompt = sqlx::query_as::<_, SystemPrompt>(
            "UPDATE system_prompts SET
                name = COALESCE($1, name),
                prompt_text = COALESCE($2, prompt_text),
                variables = COALESCE($3, variables),
                is_active = COALESCE($4, is_active),
                version = $5,
                updated_at = NOW()
             WHERE id = $6
             RETURNING *"
        )
        .bind(req.name)
        .bind(req.prompt_text)
        .bind(req.variables)
        .bind(req.is_active)
        .bind(new_version)
        .bind(prompt_id)
        .fetch_one(pool)
        .await
        .map_err(Error::new)?;
        
        Response::new().json(&prompt).map_err(Error::new)
    }
    
    async fn delete(&self, request: &Request, id: &String) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        let prompt_id = Uuid::parse_str(id).map_err(Error::new)?;
        
        let result = sqlx::query(
            "UPDATE system_prompts SET is_active = false, updated_at = NOW() WHERE id = $1"
        )
        .bind(prompt_id)
        .execute(pool)
        .await
        .map_err(Error::new)?;
        
        if result.rows_affected() == 0 {
            return Err(Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Prompt not found")));
        }
        
        Ok(Response::new())
    }
}

#[derive(Deserialize)]
pub struct UpdatePromptRequest {
    pub name: Option<String>,
    pub prompt_text: Option<String>,
    pub variables: Option<serde_json::Value>,
    pub is_active: Option<bool>,
}

#[derive(Default, macros::RestController)]
pub struct ApiKeysController;

#[async_trait]
impl RestController for ApiKeysController {
    type Resource = String;
    
    async fn list(&self, request: &Request) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        
        let keys = sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys ORDER BY created_at DESC"
        )
        .fetch_all(pool)
        .await
        .map_err(Error::new)?;
        
        Response::new().json(&keys).map_err(Error::new)
    }
    
    async fn get(&self, request: &Request, id: &String) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        let key_id = Uuid::parse_str(id).map_err(Error::new)?;
        
        let key = sqlx::query_as::<_, ApiKey>(
            "SELECT * FROM api_keys WHERE id = $1"
        )
        .bind(key_id)
        .fetch_optional(pool)
        .await
        .map_err(Error::new)?
        .ok_or_else(|| Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "API key not found")))?;
        
        Response::new().json(&key).map_err(Error::new)
    }
    
    async fn create(&self, request: &Request) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        let user_id: Uuid = RequestUserExt::user_id(request)?;
        let req: CreateApiKeyRequest = request.json().map_err(Error::new)?;
        
        let api_key = sqlx::query_as::<_, ApiKey>(
            "INSERT INTO api_keys (name, provider, api_key, base_url, created_by)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING *"
        )
        .bind(req.name)
        .bind(req.provider)
        .bind(req.api_key)
        .bind(req.base_url)
        .bind(user_id)
        .fetch_one(pool)
        .await
        .map_err(Error::new)?;
        
        Response::new().json(&api_key).map_err(Error::new)
    }
    
    async fn update(&self, request: &Request, id: &String) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let _pool = crate::db::get_pool();
        let _key_id = Uuid::parse_str(id).map_err(Error::new)?;
        
        // For now, just return not implemented
        // You can add update logic if needed
        Err(Error::new(std::io::Error::new(std::io::ErrorKind::Other, "Update not implemented")))
    }
    
    async fn delete(&self, request: &Request, id: &String) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        let key_id = Uuid::parse_str(id).map_err(Error::new)?;
        
        let result = sqlx::query(
            "DELETE FROM api_keys WHERE id = $1"
        )
        .bind(key_id)
        .execute(pool)
        .await
        .map_err(Error::new)?;
        
        if result.rows_affected() == 0 {
            return Err(Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "API key not found")));
        }
        
        Ok(Response::new())
    }
}

#[derive(Default)]
pub struct BackgroundJobsController;

#[async_trait]
impl Controller for BackgroundJobsController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        let query = request.query();
        
        let status = query.get::<String>("status");
        let job_type = query.get::<String>("job_type");
        let limit: i64 = query.get::<i64>("limit").unwrap_or(50).min(100);
        let offset: i64 = query.get::<i64>("offset").unwrap_or(0);
        
        let mut sql = String::from("SELECT * FROM background_jobs WHERE 1=1");
        
        let mut bind_index = 1;
        if status.is_some() {
            sql.push_str(&format!(" AND status = ${}", bind_index));
            bind_index += 1;
        }
        if job_type.is_some() {
            sql.push_str(&format!(" AND job_type = ${}", bind_index));
            bind_index += 1;
        }
        
        sql.push_str(&format!(" ORDER BY created_at DESC LIMIT ${} OFFSET ${}", bind_index, bind_index + 1));
        
        let mut query_builder = sqlx::query_as::<_, BackgroundJob>(&sql);
        
        if let Some(ref status) = status {
            query_builder = query_builder.bind(status);
        }
        if let Some(ref job_type) = job_type {
            query_builder = query_builder.bind(job_type);
        }
        
        let jobs = query_builder
            .bind(limit)
            .bind(offset)
            .fetch_all(pool)
            .await
            .map_err(Error::new)?;
        
        Response::new().json(&jobs).map_err(Error::new)
    }
}

#[derive(Default)]
pub struct BackgroundJobController;

#[async_trait]
impl Controller for BackgroundJobController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        // Verify admin access
        request.require_role("admin")?;
        
        let pool = crate::db::get_pool();
        
        let id_str = request.parameter::<String>("id")?.unwrap_or_default();
        let id = Uuid::parse_str(&id_str).map_err(Error::new)?;
        
        let job = sqlx::query_as::<_, BackgroundJob>(
            "SELECT * FROM background_jobs WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(Error::new)?
        .ok_or_else(|| Error::new(std::io::Error::new(std::io::ErrorKind::NotFound, "Job not found")))?;
        
        Response::new().json(&job).map_err(Error::new)
    }
}