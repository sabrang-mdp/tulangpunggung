use rwf::prelude::*;
use uuid::Uuid;
use crate::models::*;
use crate::middleware::auth::RequestUserExt;

#[derive(Default)]
pub struct DashboardStatsController;

#[async_trait]
impl Controller for DashboardStatsController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        // Verify user is authenticated
        let _user_id = RequestUserExt::user_id(request)?;
        
        let pool = crate::db::get_pool();
        
        let total_reports: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reports")
            .fetch_one(pool)
            .await
            .map_err(Error::new)?;
        
        let active_reports: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reports WHERE status IN ('submitted', 'verified', 'in_progress')"
        )
        .fetch_one(pool)
        .await
        .map_err(Error::new)?;
        
        let resolved_reports: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reports WHERE status = 'resolved'"
        )
        .fetch_one(pool)
        .await
        .map_err(Error::new)?;
        
        let total_users: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
            .fetch_one(pool)
            .await
            .map_err(Error::new)?;
        
        let reports_this_week: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reports WHERE created_at >= NOW() - INTERVAL '7 days'"
        )
        .fetch_one(pool)
        .await
        .map_err(Error::new)?;
        
        let reports_this_month: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM reports WHERE created_at >= NOW() - INTERVAL '30 days'"
        )
        .fetch_one(pool)
        .await
        .map_err(Error::new)?;
        
        let average_resolution_time_hours: Option<f64> = sqlx::query_scalar(
            "SELECT AVG(EXTRACT(EPOCH FROM (resolved_at - created_at)) / 3600)
             FROM tickets WHERE resolved_at IS NOT NULL"
        )
        .fetch_one(pool)
        .await
        .map_err(Error::new)?;
        
        let top_categories: Vec<CategoryStats> = sqlx::query_as(
            "SELECT 
                c.id as category_id,
                c.name as category_name,
                COUNT(r.id)::bigint as report_count,
                (COUNT(r.id)::float / NULLIF($1::float, 0) * 100) as percentage
             FROM categories c
             LEFT JOIN reports r ON c.id = r.category_id
             GROUP BY c.id, c.name
             ORDER BY report_count DESC
             LIMIT 5"
        )
        .bind(total_reports)
        .fetch_all(pool)
        .await
        .map_err(Error::new)?;
        
        let stats = DashboardStats {
            total_reports,
            active_reports,
            resolved_reports,
            total_users,
            reports_this_week,
            reports_this_month,
            average_resolution_time_hours,
            top_categories,
        };
        
        Response::new().json(&stats).map_err(Error::new)
    }
}

#[derive(Default)]
pub struct DashboardTrendsController;

#[async_trait]
impl Controller for DashboardTrendsController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        // Verify user is authenticated
        let _user_id = RequestUserExt::user_id(request)?;
        
        let pool = crate::db::get_pool();
        let query = request.query();
        
        let days: i32 = query.get::<i32>("days").unwrap_or(30);
        let category_id = query.get::<String>("category_id")
            .and_then(|s| Uuid::parse_str(&s).ok());
        
        let mut sql = String::from(
            "SELECT 
                TO_CHAR(DATE(created_at), 'YYYY-MM-DD') as date,
                COUNT(*)::bigint as count,
                c.name as category
             FROM reports r
             LEFT JOIN categories c ON r.category_id = c.id
             WHERE created_at >= NOW() - INTERVAL '1 day' * $1"
        );
        
        if category_id.is_some() {
            sql.push_str(" AND r.category_id = $2");
        }
        
        sql.push_str(" GROUP BY DATE(created_at), c.name ORDER BY DATE(created_at) ASC");
        
        let mut query_builder = sqlx::query_as::<_, TrendData>(&sql)
            .bind(days);
        
        if let Some(cat_id) = category_id {
            query_builder = query_builder.bind(cat_id);
        }
        
        let trends = query_builder
            .fetch_all(pool)
            .await
            .map_err(Error::new)?;
        
        Response::new().json(&trends).map_err(Error::new)
    }
}

#[derive(Default)]
pub struct DashboardClustersController;

#[async_trait]
impl Controller for DashboardClustersController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        // Verify user is authenticated
        let _user_id = RequestUserExt::user_id(request)?;
        
        let pool = crate::db::get_pool();
        
        let clusters = sqlx::query_as::<_, ReportCluster>(
            "SELECT * FROM report_clusters ORDER BY report_count DESC LIMIT 10"
        )
        .fetch_all(pool)
        .await
        .map_err(Error::new)?;
        
        Response::new().json(&clusters).map_err(Error::new)
    }
}

#[derive(Default)]
pub struct DashboardHeatmapController;

#[async_trait]
impl Controller for DashboardHeatmapController {
    async fn handle(&self, request: &Request) -> Result<Response, Error> {
        // Verify user is authenticated
        let _user_id = RequestUserExt::user_id(request)?;
        
        let pool = crate::db::get_pool();
        let query = request.query();
        
        let category_id = query.get::<String>("category_id")
            .and_then(|s| Uuid::parse_str(&s).ok());
        
        let mut sql = String::from(
            "SELECT 
                CAST(latitude AS DOUBLE PRECISION) as latitude,
                CAST(longitude AS DOUBLE PRECISION) as longitude,
                COUNT(*)::int as intensity,
                ARRAY_AGG(id) as reports
             FROM reports
             WHERE latitude IS NOT NULL AND longitude IS NOT NULL"
        );
        
        if category_id.is_some() {
            sql.push_str(" AND category_id = $1");
        }
        
        sql.push_str(" GROUP BY latitude, longitude");
        
        let mut query_builder = sqlx::query_as::<_, HeatmapPoint>(&sql);
        
        if let Some(cat_id) = category_id {
            query_builder = query_builder.bind(cat_id);
        }
        
        let points = query_builder
            .fetch_all(pool)
            .await
            .map_err(Error::new)?;
        
        Response::new().json(&points).map_err(Error::new)
    }
}