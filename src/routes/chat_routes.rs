use super::handlers;
use actix_web::web;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(web::scope("/chat").service(handlers::chat_handler::chat_ws));
}
