use rwf::prelude::*;
use rwf::job::Error as JobError;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ClusteringJob;

#[async_trait]
impl Job for ClusteringJob {
    async fn execute(&self, _args: serde_json::Value) -> Result<(), JobError> {
        let pool = crate::db::get_pool();
        
        perform_clustering(pool).await.map_err(|e| {
            tracing::error!("Clustering job failed: {:?}", e);
            JobError::from(serde_json::from_str::<serde_json::Value>("").unwrap_err())
        })?;

        Ok(())
    }    
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct CleanupJob;

#[async_trait]
impl Job for CleanupJob {
    async fn execute(&self, _args: serde_json::Value) -> Result<(), JobError> {
        let pool = crate::db::get_pool();
        
        perform_cleanup(pool).await.map_err(|e| {
            tracing::error!("Cleanup job failed: {:?}", e);
            JobError::from(serde_json::from_str::<serde_json::Value>("").unwrap_err())
        })?;
        
        Ok(())
    }
}

async fn perform_clustering(pool: &PgPool) -> Result<(), sqlx::Error> {
    let reports: Vec<(Uuid, Option<f64>, Option<f64>)> = sqlx::query_as(
        "SELECT id, CAST(latitude AS DOUBLE PRECISION), CAST(longitude AS DOUBLE PRECISION)
         FROM reports 
         WHERE latitude IS NOT NULL AND longitude IS NOT NULL 
         AND cluster_id IS NULL
         AND created_at >= NOW() - INTERVAL '30 days'"
    )
    .fetch_all(pool)
    .await?;
    
    for (report_id, lat, lon) in reports {
        if let (Some(lat), Some(lon)) = (lat, lon) {
            let nearby_cluster: Option<(Uuid,)> = sqlx::query_as(
                "SELECT id FROM report_clusters
                 WHERE center_latitude IS NOT NULL 
                 AND center_longitude IS NOT NULL
                 AND earth_distance(
                     ll_to_earth(CAST(center_latitude AS DOUBLE PRECISION), CAST(center_longitude AS DOUBLE PRECISION)),
                     ll_to_earth($1, $2)
                 ) < 1000
                 ORDER BY earth_distance(
                     ll_to_earth(CAST(center_latitude AS DOUBLE PRECISION), CAST(center_longitude AS DOUBLE PRECISION)),
                     ll_to_earth($1, $2)
                 )
                 LIMIT 1"
            )
            .bind(lat)
            .bind(lon)
            .fetch_optional(pool)
            .await?;
            
            let cluster_id = if let Some((cluster_id,)) = nearby_cluster {
                sqlx::query(
                    "UPDATE reports SET cluster_id = $1 WHERE id = $2"
                )
                .bind(cluster_id)
                .bind(report_id)
                .execute(pool)
                .await?;
                
                sqlx::query(
                    "UPDATE report_clusters 
                     SET report_count = report_count + 1,
                         updated_at = NOW()
                     WHERE id = $1"
                )
                .bind(cluster_id)
                .execute(pool)
                .await?;
                
                cluster_id
            } else {
                let new_cluster_id = Uuid::new_v4();
                
                sqlx::query(
                    "INSERT INTO report_clusters 
                     (id, center_latitude, center_longitude, report_count)
                     VALUES ($1, $2, $3, 1)"
                )
                .bind(new_cluster_id)
                .bind(lat)
                .bind(lon)
                .execute(pool)
                .await?;
                
                sqlx::query(
                    "UPDATE reports SET cluster_id = $1 WHERE id = $2"
                )
                .bind(new_cluster_id)
                .bind(report_id)
                .execute(pool)
                .await?;
                
                new_cluster_id
            };
            
            update_cluster_metadata(pool, cluster_id).await?;
        }
    }
    
    Ok(())
}

async fn update_cluster_metadata(pool: &PgPool, cluster_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE report_clusters rc
         SET 
             center_latitude = (
                 SELECT AVG(latitude) FROM reports WHERE cluster_id = rc.id
             ),
             center_longitude = (
                 SELECT AVG(longitude) FROM reports WHERE cluster_id = rc.id
             ),
             earliest_incident = (
                 SELECT MIN(incident_date) FROM reports WHERE cluster_id = rc.id
             ),
             latest_incident = (
                 SELECT MAX(incident_date) FROM reports WHERE cluster_id = rc.id
             ),
             report_count = (
                 SELECT COUNT(*) FROM reports WHERE cluster_id = rc.id
             )
         WHERE id = $1"
    )
    .bind(cluster_id)
    .execute(pool)
    .await?;
    
    Ok(())
}

async fn perform_cleanup(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM background_jobs 
         WHERE status = 'completed' 
         AND completed_at < NOW() - INTERVAL '30 days'"
    )
    .execute(pool)
    .await?;
    
    sqlx::query(
        "DELETE FROM chat_sessions 
         WHERE status = 'archived' 
         AND updated_at < NOW() - INTERVAL '90 days'"
    )
    .execute(pool)
    .await?;
    
    Ok(())
}