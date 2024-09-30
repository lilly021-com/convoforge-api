use super::handlers;
use crate::middlewares;
use actix_web::web;
use actix_web_lab::middleware::from_fn;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/user")
            .wrap(from_fn(middlewares::auth_middleware::check_auth_middleware))
            .service(handlers::user_handler::get_users_by_page)
            .service(handlers::user_handler::get_users_in_channel)
            .service(handlers::user_handler::get_current_user)
            .service(handlers::user_handler::purge_user)
            .service(handlers::user_handler::update_display_name)
            .service(handlers::user_handler::update_profile_image),
    );
}
