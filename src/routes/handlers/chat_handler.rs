use actix_web::{get, web, Error, HttpRequest, HttpResponse, Result};
use actix_web_actors::ws;
use std::collections::HashMap;
use std::sync::Arc;

use crate::utils::chat::{ChatRoom, MyWebSocket};
use crate::utils::jwt::get_user_id_from_token;

#[get("/ws")]
pub async fn chat_ws(
    req: HttpRequest,
    stream: web::Payload,
    room: web::Data<Arc<ChatRoom>>,
    query: web::Query<HashMap<String, String>>,
) -> Result<HttpResponse, Error> {
    let token = match query.get("token") {
        Some(token) => token,
        None => {
            return Err(actix_web::error::ErrorBadRequest("Token is required"));
        }
    };

    let user_id = match get_user_id_from_token(token.to_string()) {
        Ok(user_id) => user_id,
        Err(_) => {
            return Err(actix_web::error::ErrorBadRequest("Invalid token"));
        }
    };

    let ws = MyWebSocket {
        room: room.get_ref().clone(),
        user_id,
    };
    ws::start(ws, &req, stream)
}
