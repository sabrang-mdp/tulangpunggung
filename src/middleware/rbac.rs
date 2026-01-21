use rwf::controller::middleware::prelude::*;
use rwf::controller::Middleware;
use rwf::http::{Request, Response};
use crate::middleware::auth::RequestUserExt;

#[derive(Debug, Clone)]
pub struct RoleMiddleware;

impl RoleMiddleware {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Middleware for RoleMiddleware {
    async fn handle_request(&self, request: Request) -> Result<Outcome, Error> {
        let path = request.path().path();

        // Define protected routes and required roles
        let admin_routes = vec![
            "/api/admin/",
            "/api/dashboard/",
        ];

        let moderator_routes = vec![
            "/api/tickets/",
        ];

        // Check if route requires admin
        if admin_routes.iter().any(|prefix| path.starts_with(prefix)) {
            match request.get_user() {
                Ok(user) => {
                    if user.role != "admin" && user.role != "moderator" {
                        let response = Response::new()
                            .html("<h1>Admin access Required</h1>");
                        return Ok(Outcome::Stop(request, response));
                    }
                }
                Err(_) => {
                    let response = Response::new()
                        .html("<h1>Unauthorized</h1>");
                    return Ok(Outcome::Stop(request, response));
                }
            }
        }

        // Check if route requires moderator
        if moderator_routes.iter().any(|prefix| path.starts_with(prefix)) {
            match request.get_user() {
                Ok(user) => {
                    if user.role != "admin" && user.role != "moderator" {
                        let response = Response::new()
                            .html("<h1>Moderator access Required</h1>");
                        return Ok(Outcome::Stop(request, response));
                    }
                }
                Err(_) => {
                    let response = Response::new()
                        .html("<h1>Unauthorized</h1>");
                    return Ok(Outcome::Stop(request, response));
                }
            }
        }

        Ok(Outcome::Forward(request))
    }
}