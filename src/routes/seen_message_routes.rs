use super::handlers;
use crate::middlewares;
use actix_web::web;
use actix_web_lab::middleware::from_fn;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/seen")
            .wrap(from_fn(middlewares::auth_middleware::check_auth_middleware))
            .service(handlers::seen_message_handler::set_seen_message)
            .service(handlers::seen_message_handler::get_seen_users_by_message_id),
    );
}
