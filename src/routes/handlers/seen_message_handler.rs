use crate::utils::api_response::ApiResponse;
use crate::utils::app_state;
use crate::utils::jwt::get_user_id_from_http_request;
use actix_web::{get, post, web, HttpRequest};
use entity::{message, seen_message};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct MessageIdsDTO {
    ids: Vec<Uuid>,
}

#[derive(Serialize, Deserialize)]
struct MessageDTO {
    id: Uuid,
    user_id: Uuid,
    content: Option<String>,
    date_created: String,
    date_updated: String,
    message_type: String,
    recipient_type: String,
    reference_id: Uuid,
    deleted: bool,
}

impl From<message::Model> for MessageDTO {
    fn from(model: message::Model) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            content: model.content,
            date_created: model.date_created.to_string(),
            date_updated: model.date_updated.to_string(),
            message_type: model.message_type.to_string(),
            recipient_type: model.recipient_type.to_string(),
            reference_id: model.reference_id,
            deleted: model.deleted,
        }
    }
}

#[post("/")]
async fn set_seen_message(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    message_ids_dto: web::Json<MessageIdsDTO>,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req)?;

    let message_ids = message_ids_dto.ids.clone();

    for message_id in message_ids {
        let message = message::Entity::find()
            .filter(message::Column::Id.eq(message_id))
            .filter(message::Column::Deleted.eq(false))
            .one(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;

        if message.is_none() {
            continue;
        }

        let seen_message_entity = seen_message::Entity::find()
            .filter(seen_message::Column::UserId.eq(user_id))
            .filter(seen_message::Column::MessageId.eq(message_id))
            .one(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;

        if seen_message_entity.is_some() {
            continue;
        }

        let new_seen_message = seen_message::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            message_id: Set(message_id),
            date_seen: Set(chrono::Utc::now().naive_utc()),
            ..Default::default()
        };

        new_seen_message
            .insert(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;
    }

    Ok(ApiResponse::new(
        200,
        "Marked applicable messages".to_string(),
    ))
}

#[get("/")]
async fn get_seen_users_by_message_id(
    app_state: web::Data<app_state::AppState>,
    query: web::Query<HashMap<String, String>>,
) -> Result<ApiResponse, ApiResponse> {
    let message_id = query
        .get("message_id")
        .unwrap_or(&Uuid::nil().to_string())
        .parse::<Uuid>()
        .unwrap_or(Uuid::nil());

    let message = message::Entity::find()
        .filter(message::Column::Id.eq(message_id))
        .filter(message::Column::Deleted.eq(false))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if message.is_none() {
        return Err(ApiResponse::new(404, "Message not found".to_string()));
    }

    let seen_messages = seen_message::Entity::find()
        .filter(seen_message::Column::MessageId.eq(message_id))
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    let user_ids: Vec<Uuid> = seen_messages.into_iter().map(|sm| sm.user_id).collect();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&user_ids).unwrap(),
    ))
}
