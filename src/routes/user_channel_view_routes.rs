use super::handlers;
use crate::middlewares;
use actix_web::web;
use actix_web_lab::middleware::from_fn;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/user-channel-view")
            .wrap(from_fn(middlewares::auth_middleware::check_auth_middleware))
            .service(handlers::user_channel_view_handler::set_channel_last_viewed)
            .service(handlers::user_channel_view_handler::get_unread_channels),
    );
}
