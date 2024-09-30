use super::handlers;
use crate::middlewares;
use actix_web::web;
use actix_web_lab::middleware::from_fn;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/role")
            .wrap(from_fn(middlewares::auth_middleware::check_auth_middleware))
            .service(handlers::role_handler::create_role)
            .service(handlers::role_handler::get_all)
            .service(handlers::role_handler::update_role)
            .service(handlers::role_handler::delete_role),
    );
}
