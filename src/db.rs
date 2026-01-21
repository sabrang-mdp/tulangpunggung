use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgPool};
use std::sync::OnceLock;
use std::time::Duration;
use crate::config::Config;

static POOL: OnceLock<PgPool> = OnceLock::new();

pub async fn create_pool(config: &Config) -> Result<PgPool, sqlx::Error> {
    let options = PgConnectOptions::new()
        .host(&config.db_host)
        .port(config.db_port)
        .username(&config.db_user)
        .password(&config.db_pass) 
        .database(&config.db_name);

    PgPoolOptions::new()
        .max_connections(20)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(30))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect_with(options)
        .await
}

pub async fn init_pool(config: &Config) -> Result<PgPool, sqlx::Error> {
    let pool = create_pool(config).await?;
    POOL.set(pool.clone()).map_err(|_| sqlx::Error::Configuration("Already init".into()))?;
    Ok(pool)
}

pub fn get_pool() -> &'static PgPool {
    POOL.get().expect("Pool not initialized. Call init_pool first.")
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
}