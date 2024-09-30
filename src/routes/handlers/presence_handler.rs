use crate::utils::api_response::ApiResponse;
use crate::utils::app_state;
use crate::utils::chat::ChatRoom;
use crate::utils::jwt::get_user_id_from_http_request;
use crate::utils::organization_util::get_organization_id_from_user_id;
use actix_web::{get, post, web, HttpRequest};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct ChannelIndicateDTO {
    reference_id: Uuid,
    recipient_type: String,
}

#[derive(Serialize, Deserialize)]
struct ChannelIndicateResponseDTO {
    message_type: String,
    user_id: Uuid,
    reference_id: Uuid,
    recipient_type: String,
}

#[get("/")]
pub async fn get_all(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    chat_room: web::Data<Arc<ChatRoom>>,
) -> actix_web::Result<ApiResponse, ApiResponse> {
    let user_ids = chat_room.get_connected_user_ids();

    let current_user_id = get_user_id_from_http_request(req)?;

    let user_organization_id =
        get_organization_id_from_user_id(&app_state.db, current_user_id).await?;

    //for each user id, exclude them from the list if they do not have the same organization id

    let mut filtered_user_ids: Vec<Uuid> = Vec::new();

    for user_id in user_ids {
        let organization_id = get_organization_id_from_user_id(&app_state.db, user_id).await?;

        if organization_id == user_organization_id {
            filtered_user_ids.push(user_id);
        }
    }

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&filtered_user_ids).unwrap(),
    ))
}

#[post("/typing")]
pub async fn send_typing_indicator_to_channel_id(
    app_state: web::Data<app_state::AppState>,
    chat_room: web::Data<Arc<ChatRoom>>,
    dto: web::Json<ChannelIndicateDTO>,
    req: HttpRequest,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req)?;

    let reference_id = dto.reference_id;
    let recipient_type = dto.recipient_type.clone();

    if recipient_type != "CHANNEL" && recipient_type != "USER" {
        return Err(ApiResponse::new(400, "Invalid recipient type".to_string()));
    }

    let mut user_ids: HashSet<Uuid> = HashSet::new();

    if recipient_type == "CHANNEL" {
        let channel_role_accesses = entity::channel_role_access::Entity::find()
            .filter(entity::channel_role_access::Column::ChannelId.eq(reference_id))
            .filter(entity::channel_role_access::Column::Deleted.eq(false))
            .all(&app_state.db)
            .await
            .unwrap();

        for channel_role_access in channel_role_accesses {
            let user_role_accesses = entity::user_role_access::Entity::find()
                .filter(entity::user_role_access::Column::RoleId.eq(channel_role_access.role_id))
                .filter(entity::user_role_access::Column::Deleted.eq(false))
                .all(&app_state.db)
                .await
                .unwrap();

            for user_role_access in user_role_accesses {
                user_ids.insert(user_role_access.user_id);
            }
        }

        let admin_and_manage_channel_roles = entity::role::Entity::find()
            .filter(
                entity::role::Column::Administrator
                    .eq(true)
                    .or(entity::role::Column::ManageChannels.eq(true)),
            )
            .all(&app_state.db)
            .await
            .unwrap();

        let admin_and_manage_channel_role_ids: Vec<Uuid> = admin_and_manage_channel_roles
            .into_iter()
            .map(|role| role.id)
            .collect();

        let admin_and_manage_channel_user_role_accesses = entity::user_role_access::Entity::find()
            .filter(
                entity::user_role_access::Column::RoleId.is_in(admin_and_manage_channel_role_ids),
            )
            .filter(entity::user_role_access::Column::Deleted.eq(false))
            .all(&app_state.db)
            .await
            .unwrap();

        for user_role_access in admin_and_manage_channel_user_role_accesses {
            user_ids.insert(user_role_access.user_id);
        }
    }

    if recipient_type == "USER" {
        user_ids.insert(reference_id);
    }

    let indicate_dto = ChannelIndicateResponseDTO {
        message_type: "TYPING".to_string(),
        user_id,
        reference_id,
        recipient_type,
    };

    chat_room.send_message(
        &user_ids.into_iter().collect::<Vec<Uuid>>(),
        &serde_json::to_string(&indicate_dto).unwrap(),
    );

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&indicate_dto).unwrap(),
    ))
}
