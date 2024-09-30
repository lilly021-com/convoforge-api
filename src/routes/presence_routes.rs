use super::handlers;
use crate::middlewares;
use actix_web::web;
use actix_web_lab::middleware::from_fn;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/presence")
            .wrap(from_fn(middlewares::auth_middleware::check_auth_middleware))
            .service(handlers::presence_handler::get_all)
            .service(handlers::presence_handler::send_typing_indicator_to_channel_id),
    );
}
