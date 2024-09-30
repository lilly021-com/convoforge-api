use super::handlers;
use crate::middlewares;
use actix_web::web;
use actix_web_lab::middleware::from_fn;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/channel")
            .wrap(from_fn(middlewares::auth_middleware::check_auth_middleware))
            .service(handlers::channel_handler::get_my)
            .service(handlers::channel_handler::create_channel)
            .service(handlers::channel_handler::delete_channel)
            .service(handlers::channel_handler::update_channel),
    );
}
