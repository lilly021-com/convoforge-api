use super::handlers;
use crate::middlewares;
use actix_web::web;
use actix_web_lab::middleware::from_fn;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/message")
            .wrap(from_fn(middlewares::auth_middleware::check_auth_middleware))
            .service(handlers::message_handler::send_message)
            .service(handlers::message_handler::get_by_channel_and_per_page)
            .service(handlers::message_handler::edit_message)
            .service(handlers::message_handler::delete_message)
            .service(handlers::message_handler::search_messages),
    );
}
