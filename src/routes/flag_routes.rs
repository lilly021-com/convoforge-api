use super::handlers;
use crate::middlewares;
use actix_web::web;
use actix_web_lab::middleware::from_fn;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/flag")
            .wrap(from_fn(middlewares::auth_middleware::check_auth_middleware))
            .service(handlers::flag_handler::update_flag),
    );
}
