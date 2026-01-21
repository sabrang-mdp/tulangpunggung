use rwf::prelude::*;
use rwf::http::{Request, Response};
use rwf::controller::Error;
use serde_json::json;

#[derive(Default)]
pub struct HealthCheckController;

#[async_trait]
impl Controller for HealthCheckController {
    async fn handle(&self, _request: &Request) -> Result<Response, Error> {
        let health_status = json!({
            "status": "healthy",
            "service": "TulangPunggung API",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": chrono::Utc::now(),
        });

        Response::new().json(&health_status).map_err(Error::new)
    }
}