use super::handlers;
use crate::middlewares;
use actix_web::web;
use actix_web_lab::middleware::from_fn;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/channel-role-access")
            .wrap(from_fn(middlewares::auth_middleware::check_auth_middleware))
            .service(handlers::channel_role_access_handler::create_channel_role_access)
            .service(handlers::channel_role_access_handler::get_all)
            .service(handlers::channel_role_access_handler::delete),
    );
}
