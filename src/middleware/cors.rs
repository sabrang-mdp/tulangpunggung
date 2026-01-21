use rwf::prelude::*;
use rwf::controller::Middleware;
use rwf::controller::Outcome;

#[derive(Debug, Clone)]
pub struct CorsMiddleware {
    allowed_origins: Vec<String>,
}

impl CorsMiddleware {
    pub fn new() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
        }
    }

    pub fn with_origins(origins: Vec<String>) -> Self {
        Self {
            allowed_origins: origins,
        }
    }
}

#[async_trait]
impl Middleware for CorsMiddleware {
    async fn handle_request(&self, request: Request) -> Result<Outcome, Error> {
        // Handle OPTIONS preflight request
        if request.method().to_string() == "OPTIONS" {
            let origin = request
                .headers()
                .get("origin")
                .or_else(|| request.headers().get("Origin"))
                .map(|s| s.as_str())
                .unwrap_or("*");

            let response = Response::new()
                .header("Access-Control-Allow-Origin", origin)
                .header("Access-Control-Allow-Methods", "GET, POST, PUT, PATCH, DELETE, OPTIONS")
                .header("Access-Control-Allow-Headers", "Content-Type, Authorization")
                .header("Access-Control-Allow-Credentials", "true")
                .header("Access-Control-Max-Age", "86400");
            
            return Ok(Outcome::Stop(request, response));
        }

        Ok(Outcome::Forward(request))
    }

    async fn handle_response(&self, request: &Request, response: Response) -> Result<Response, Error> {
        let origin = request
            .headers()
            .get("origin")
            .or_else(|| request.headers().get("Origin"))
            .map(|s| s.as_str())
            .unwrap_or("*");

        let response = response
            .header("Access-Control-Allow-Origin", origin)
            .header("Access-Control-Allow-Methods", "GET, POST, PUT, PATCH, DELETE, OPTIONS")
            .header("Access-Control-Allow-Headers", "Content-Type, Authorization")
            .header("Access-Control-Allow-Credentials", "true")
            .header("Access-Control-Max-Age", "86400");

        Ok(response)
    }
}