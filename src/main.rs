mod config;
mod models;
mod handlers;
mod services;
mod middleware;
mod websocket;
mod background;
mod db;
mod error;

use rwf::prelude::*;
use rwf::http::Server;
use rwf::job::Worker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Logger::init();

    let config = config::Config::from_env()?;
    unsafe {
        std::env::set_var("DATABASE_URL", config.build_rwf_db_url());
        std::env::set_var("RWF_HOST", &config.host);
        std::env::set_var("RWF_PORT", config.port.to_string());
    }

    let db_pool = db::init_pool(&config).await?;

    rwf_admin::install()?;
    
    db::run_migrations(&db_pool).await?;
    
    let schedule = vec![
        background::jobs::ClusteringJob::default().schedule(
            serde_json::Value::Null,
            "0 * * * *",
        ).unwrap(),
        background::jobs::CleanupJob::default().schedule(
            serde_json::Value::Null,
            "0 0 * * *",
        ).unwrap(),
    ];

    let worker = Worker::new(vec![
        background::jobs::ClusteringJob::default().job(),
        background::jobs::CleanupJob::default().job(),
    ])
    .clock(schedule);

    worker.start().await?;
    
    let mut routes = vec![
        route!("/health" => handlers::health::HealthCheckController),
        
        route!("/ws/chat" => websocket::handler::ChatWebSocketController),

        route!("/chat/sessions" => handlers::chat::ChatSessionsController),
        route!("/chat/sessions/:id" => handlers::chat::ChatSessionController),
        route!("/chat/sessions/:id/messages" => handlers::chat::ChatMessagesController),
        
        route!("/reports" => handlers::reports::ReportsController),
        route!("/reports/:id/complete" => handlers::reports::ReportCompleteController),
        
        route!("/tickets" => handlers::tickets::TicketsListController),
        route!("/tickets/:id" => handlers::tickets::TicketController),
        route!("/tickets/:id/comments" => handlers::tickets::TicketCommentsController),
        route!("/tickets/:id/status" => handlers::tickets::TicketStatusController),    
        
        route!("/dashboard/stats" => handlers::dashboard::DashboardStatsController),
        route!("/dashboard/trends" => handlers::dashboard::DashboardTrendsController),
        route!("/dashboard/clusters" => handlers::dashboard::DashboardClustersController),
        route!("/dashboard/heatmap" => handlers::dashboard::DashboardHeatmapController),
        
        route!("/panel/users" => handlers::panel::AdminUsersController),
        route!("/panel/users/:id/role" => handlers::panel::AdminUserRoleController),
        route!("/panel/categories" => handlers::panel::CategoriesController),
        route!("/panel/prompts" => handlers::panel::PromptsController),
        route!("/panel/api-keys" => handlers::panel::ApiKeysController),
        route!("/panel/jobs" => handlers::panel::BackgroundJobsController),
        route!("/panel/jobs/:id" => handlers::panel::BackgroundJobController),
    ];

    routes.extend(rwf_admin::routes()?);

    Server::new(routes)
        .launch()       
        .await?;
    
    Ok(())
}