use super::handlers;
use crate::middlewares;
use actix_web::web;
use actix_web_lab::middleware::from_fn;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/media")
            .wrap(from_fn(middlewares::auth_middleware::check_auth_middleware))
            .wrap(from_fn(middlewares::media_middleware::get_s3_flag_on))
            .service(handlers::media_handler::upload_media)
            .service(handlers::media_handler::get_all_media_by_message_id)
            .service(handlers::media_handler::delete_media)
            .service(handlers::media_handler::get_media_by_list_of_message_ids),
    );
}
