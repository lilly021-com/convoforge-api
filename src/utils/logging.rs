use crate::utils::jwt::get_user_id_from_http_request;
use actix_web::HttpRequest;
use log::info;

pub fn log_info(req: HttpRequest, message: String) {
    let user_id = get_user_id_from_http_request(req).unwrap();
    info!("User ID: {} - {}", user_id, message);
}
