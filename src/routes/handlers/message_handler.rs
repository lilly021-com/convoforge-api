use crate::utils::api_response::ApiResponse;
use crate::utils::app_state;
use crate::utils::chat::ChatRoom;
use crate::utils::jwt::get_user_id_from_http_request;
use crate::utils::logging::log_info;
use crate::utils::permissions::{
    check_chat_permission, check_permission, ChatPermission, Permission,
};
use actix_web::{delete, get, patch, post, web, HttpRequest, Result};
use chrono::Utc;
use entity::{
    channel, channel_role_access, media, message, role, user, user_channel_view, user_role_access,
};
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct MessageSendDTO {
    content: Option<String>,
    message_type: String,
    recipient_type: String,
    reference_id: Uuid,
    media_ids: Vec<Uuid>,
}

#[derive(Serialize, Deserialize)]
struct MessageEditDTO {
    id: Uuid,
    content: Option<String>,
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
pub async fn send_message(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    message_send_dto: web::Json<MessageSendDTO>,
    chat_room: web::Data<Arc<ChatRoom>>,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req.clone())?;

    match message_send_dto.recipient_type.as_str() {
        "CHANNEL" => {
            let has_user_access_to_channel = get_user_has_access_to_channel(
                app_state.clone(),
                user_id,
                message_send_dto.reference_id,
            )
            .await?;

            if !has_user_access_to_channel {
                return Err(ApiResponse::new(
                    400,
                    "User does not have access to channel".to_string(),
                ));
            }

            let user_can_write = check_chat_permission(
                &app_state.db,
                req,
                ChatPermission::CanWrite,
                message_send_dto.reference_id,
            )
            .await;

            if !user_can_write {
                return Err(ApiResponse::new(
                    403,
                    "You do not have permission to write to this channel.".to_string(),
                ));
            }
        }
        "USER" => {
            //implement checks to see if a user requires permission to write to another user... for now not necessary
        }
        _ => {
            return Err(ApiResponse::new(
                400,
                "Recipient type must be either CHANNEL or USER.".to_string(),
            ));
        }
    }

    // Now that permissions are verified, proceed with message creation

    for media_id in message_send_dto.media_ids.iter() {
        let media_model = media::Entity::find()
            .filter(media::Column::Id.eq(*media_id))
            .filter(media::Column::UserId.eq(user_id))
            .one(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;

        if media_model.is_none() {
            return Err(ApiResponse::new(400, "Media not found".to_string()));
        }
    }

    let message_model = message::ActiveModel {
        id: Set(Uuid::new_v4()),
        content: Set(message_send_dto.content.clone()),
        user_id: Set(user_id),
        message_type: Set(message_send_dto.message_type.clone()),
        recipient_type: Set(message_send_dto.recipient_type.clone()),
        reference_id: Set(message_send_dto.reference_id),
        date_updated: Set(Utc::now().naive_utc()),
        date_created: Set(Utc::now().naive_utc()),
        deleted: Set(false),
        ..Default::default()
    }
    .insert(&app_state.db)
    .await
    .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    for media_id in message_send_dto.media_ids.iter() {
        let media_model = media::Entity::find()
            .filter(media::Column::Id.eq(*media_id))
            .one(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;

        let mut media_model = media_model.unwrap().into_active_model();
        media_model.message_id = Set(Some(message_model.id));

        media_model
            .update(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;
    }

    let user_channel_view = user_channel_view::Entity::find()
        .filter(user_channel_view::Column::UserId.eq(user_id))
        .filter(
            user_channel_view::Column::RecipientType.eq(message_send_dto.recipient_type.clone()),
        )
        .filter(user_channel_view::Column::ReferenceId.eq(message_send_dto.reference_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if user_channel_view.is_none() {
        let new_user_channel_view = user_channel_view::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            recipient_type: Set(message_send_dto.recipient_type.clone()),
            reference_id: Set(message_send_dto.reference_id),
            last_viewed: Set(Utc::now().naive_utc()),
            ..Default::default()
        };

        new_user_channel_view
            .insert(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;
    } else {
        let user_channel_view = user_channel_view.unwrap();

        let mut user_channel_view = user_channel_view.into_active_model();
        user_channel_view.last_viewed = Set(Utc::now().naive_utc());
        user_channel_view.user_id = Set(user_id);
        user_channel_view.recipient_type = Set(message_send_dto.recipient_type.clone());
        user_channel_view.reference_id = Set(message_send_dto.reference_id);

        user_channel_view
            .update(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;
    }

    let response_dto: MessageDTO = message_model.clone().into();

    match message_send_dto.recipient_type.as_str() {
        "CHANNEL" => {
            let user_ids =
                get_array_of_users_by_channel_id(app_state, message_send_dto.reference_id)
                    .await
                    .map_err(|e| ApiResponse::new(500, e.to_string()))?;

            chat_room.send_message(&user_ids, &serde_json::to_string(&response_dto).unwrap());
        }
        "USER" => {
            let user_ids = vec![message_send_dto.reference_id, user_id];
            chat_room.send_message(&user_ids, &serde_json::to_string(&response_dto).unwrap());
        }
        _ => {}
    }

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dto).unwrap(),
    ))
}

#[get("/")]
async fn get_by_channel_and_per_page(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    query: web::Query<HashMap<String, String>>,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req)?;

    let recipient_type = query
        .get("recipient_type")
        .unwrap_or(&"CHANNEL".to_string())
        .to_string();

    let reference_id = query
        .get("reference_id")
        .ok_or(ApiResponse::new(
            400,
            "Reference ID is required.".to_string(),
        ))?
        .parse::<Uuid>()
        .map_err(|e| ApiResponse::new(400, e.to_string()))?;

    let page = query
        .get("page")
        .unwrap_or(&"1".to_string())
        .parse::<u64>()
        .unwrap_or(1);

    let per_page = query
        .get("per_page")
        .unwrap_or(&"30".to_string())
        .parse::<u64>()
        .unwrap_or(30);

    let mut query = message::Entity::find()
        .filter(message::Column::MessageType.eq("MESSAGE"))
        .filter(message::Column::RecipientType.eq(recipient_type.clone()))
        .filter(message::Column::Deleted.eq(false))
        .order_by_desc(message::Column::DateCreated);

    if recipient_type == "USER" {
        query = query.filter(
            message::Column::UserId
                .eq(user_id)
                .and(message::Column::ReferenceId.eq(reference_id))
                .or(message::Column::UserId
                    .eq(reference_id)
                    .and(message::Column::ReferenceId.eq(user_id))),
        );
    }

    if recipient_type == "CHANNEL" {
        query = query.filter(message::Column::ReferenceId.eq(reference_id));

        let has_user_access_to_channel =
            get_user_has_access_to_channel(app_state.clone(), user_id, reference_id).await?;

        if !has_user_access_to_channel {
            return Err(ApiResponse::new(
                400,
                "User does not have access to channel".to_string(),
            ));
        }
    }

    // Paginate the combined query
    let paginator = query.paginate(&app_state.db, per_page);

    let messages = paginator
        .fetch_page(page - 1)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    let response_dtos: Vec<MessageDTO> = messages.into_iter().map(MessageDTO::from).collect();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dtos).unwrap(),
    ))
}

#[patch("/")]
async fn edit_message(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    message_edit_dto: web::Json<MessageEditDTO>,
    chat_room: web::Data<Arc<ChatRoom>>,
) -> Result<ApiResponse, ApiResponse> {
    let message_id = message_edit_dto.id;
    let user_id = get_user_id_from_http_request(req.clone())?;

    let message_model = message::Entity::find()
        .filter(message::Column::Id.eq(message_id))
        .filter(message::Column::Deleted.eq(false))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if message_model.is_none() {
        return Err(ApiResponse::new(400, "Message not found".to_string()));
    }

    let message_model = message_model.unwrap();

    if message_model.message_type != "MESSAGE" {
        return Err(ApiResponse::new(
            400,
            "Message type is not MESSAGE".to_string(),
        ));
    }

    if message_model.user_id != user_id {
        return Err(ApiResponse::new(
            400,
            "Message does not belong to user".to_string(),
        ));
    }

    if message_edit_dto.content.is_none() || message_edit_dto.content.clone().unwrap().is_empty() {
        return Err(ApiResponse::new(
            400,
            "Content must be at least 1 character".to_string(),
        ));
    }

    let response_dto = MessageDTO {
        id: message_model.id,
        user_id: message_model.user_id,
        content: message_edit_dto.content.clone(),
        date_created: message_model.date_created.to_string(),
        date_updated: Utc::now().naive_utc().to_string(),
        message_type: "EDIT_MESSAGE".to_string(),
        recipient_type: message_model.recipient_type.to_string(),
        reference_id: message_model.reference_id,
        deleted: message_model.deleted,
    };

    match message_model.recipient_type.as_str() {
        "CHANNEL" => {
            let has_user_access_to_channel = get_user_has_access_to_channel(
                app_state.clone(),
                user_id,
                message_model.reference_id,
            )
            .await?;

            if !has_user_access_to_channel {
                return Err(ApiResponse::new(
                    400,
                    "User does not have access to channel".to_string(),
                ));
            }

            let user_can_read = check_chat_permission(
                &app_state.db,
                req.clone(),
                ChatPermission::CanRead,
                message_model.reference_id,
            )
            .await;

            if !user_can_read {
                return Err(ApiResponse::new(
                    403,
                    "You do not have permission to read this channel.".to_string(),
                ));
            }

            let user_ids =
                get_array_of_users_by_channel_id(app_state.clone(), message_model.reference_id)
                    .await
                    .map_err(|e| ApiResponse::new(500, e.to_string()))?;

            chat_room.send_message(&user_ids, &serde_json::to_string(&response_dto).unwrap());
        }
        "USER" => {
            let user_ids = vec![message_model.reference_id, user_id];
            chat_room.send_message(&user_ids, &serde_json::to_string(&response_dto).unwrap());
        }
        _ => {
            return Err(ApiResponse::new(
                400,
                "Recipient type must be either CHANNEL or USER.".to_string(),
            ));
        }
    }

    let active_model = message::ActiveModel {
        id: Set(message_id),
        content: Set(message_edit_dto.content.clone()),
        date_updated: Set(Utc::now().naive_utc()),
        ..Default::default()
    };

    active_model
        .update(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    log_info(req.clone(), format!("Edited message {}", message_id));

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dto).unwrap(),
    ))
}

#[delete("/")]
async fn delete_message(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    message_edit_dto: web::Json<MessageEditDTO>,
    chat_room: web::Data<Arc<ChatRoom>>,
) -> Result<ApiResponse, ApiResponse> {
    let message_id = message_edit_dto.id;
    let user_id = get_user_id_from_http_request(req.clone())?;

    let message_model = message::Entity::find()
        .filter(message::Column::Id.eq(message_id))
        .filter(message::Column::Deleted.eq(false))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if message_model.is_none() {
        return Err(ApiResponse::new(400, "Message not found".to_string()));
    }

    let message_model = message_model.unwrap();

    let response_dto = MessageDTO {
        id: message_model.id,
        user_id: message_model.user_id,
        content: message_edit_dto.content.clone(),
        date_created: message_model.date_created.to_string(),
        date_updated: Utc::now().naive_utc().to_string(),
        message_type: "DELETE_MESSAGE".to_string(),
        recipient_type: message_model.recipient_type.to_string(),
        reference_id: message_model.reference_id,
        deleted: true,
    };

    if message_model.message_type != "MESSAGE" {
        return Err(ApiResponse::new(
            400,
            "Message type is not MESSAGE".to_string(),
        ));
    }

    let has_manage_channels =
        check_permission(&app_state.db, req.clone(), Permission::ManageChannels).await;

    if message_model.user_id != user_id && !has_manage_channels {
        return Err(ApiResponse::new(
            400,
            "Message does not belong to user".to_string(),
        ));
    }

    match message_model.recipient_type.as_str() {
        "CHANNEL" => {
            let has_user_access_to_channel = get_user_has_access_to_channel(
                app_state.clone(),
                user_id,
                message_model.reference_id,
            )
            .await?;

            if !has_user_access_to_channel {
                return Err(ApiResponse::new(
                    400,
                    "User does not have access to channel".to_string(),
                ));
            }

            let user_ids =
                get_array_of_users_by_channel_id(app_state.clone(), message_model.reference_id)
                    .await
                    .map_err(|e| ApiResponse::new(500, e.to_string()))?;

            chat_room.send_message(&user_ids, &serde_json::to_string(&response_dto).unwrap());
        }
        "USER" => {
            let user_ids = vec![message_model.reference_id, user_id];
            chat_room.send_message(&user_ids, &serde_json::to_string(&response_dto).unwrap());
        }
        _ => {
            return Err(ApiResponse::new(
                400,
                "Recipient type must be either CHANNEL or USER.".to_string(),
            ));
        }
    }

    let active_model = message::ActiveModel {
        id: Set(message_id),
        date_updated: Set(Utc::now().naive_utc()),
        content: Set(Some("DELETED".to_string())),
        deleted: Set(true),
        ..Default::default()
    };

    active_model
        .update(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    log_info(req, format!("Deleted message {}", message_id));

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dto).unwrap(),
    ))
}

async fn get_array_of_users_by_channel_id(
    app_state: web::Data<app_state::AppState>,
    channel_id: Uuid,
) -> Result<Vec<Uuid>, ApiResponse> {
    let role_ids: Vec<Uuid> = channel_role_access::Entity::find()
        .filter(channel_role_access::Column::ChannelId.eq(channel_id))
        .filter(channel_role_access::Column::Deleted.eq(false))
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?
        .into_iter()
        .map(|role_access| role_access.role_id)
        .collect();

    let mut user_ids: HashSet<Uuid> = user_role_access::Entity::find()
        .filter(user_role_access::Column::RoleId.is_in(role_ids.clone()))
        .find_also_related(user::Entity)
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?
        .into_iter()
        .filter(|(_user_role_access, user_opt)| {
            if let Some(user) = user_opt {
                !user.deleted
            } else {
                false
            }
        })
        .map(|(user_role_access, _user)| user_role_access.user_id)
        .collect();

    let admin_manage_roles: Vec<Uuid> = role::Entity::find()
        .filter(
            role::Column::Administrator
                .eq(true)
                .or(role::Column::ManageChannels.eq(true)),
        )
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?
        .into_iter()
        .map(|role| role.id)
        .collect();

    let admin_manage_user_ids: Vec<Uuid> = user_role_access::Entity::find()
        .filter(user_role_access::Column::RoleId.is_in(admin_manage_roles))
        .find_also_related(user::Entity)
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?
        .into_iter()
        .filter(|(_user_role_access, user_opt)| {
            if let Some(user) = user_opt {
                !user.deleted
            } else {
                false
            }
        })
        .map(|(user_role_access, _user)| user_role_access.user_id)
        .collect();

    user_ids.extend(admin_manage_user_ids);

    let unique_user_ids: Vec<Uuid> = user_ids.into_iter().collect();

    Ok(unique_user_ids)
}

pub async fn get_user_has_access_to_channel(
    app_state: web::Data<app_state::AppState>,
    user_id: Uuid,
    channel_id: Uuid,
) -> Result<bool, ApiResponse> {
    let user_organization_id = user::Entity::find()
        .filter(user::Column::Id.eq(user_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?
        .ok_or(ApiResponse::new(404, "User not found".to_string()))?
        .organization_id;

    let channel_organization_id = channel::Entity::find()
        .filter(channel::Column::Id.eq(channel_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?
        .ok_or(ApiResponse::new(404, "Channel not found".to_string()))?
        .organization_id;

    if user_organization_id != channel_organization_id {
        return Ok(false);
    }

    let role_ids: Vec<Uuid> = channel_role_access::Entity::find()
        .filter(
            Condition::all()
                .add(channel_role_access::Column::ChannelId.eq(channel_id))
                .add(channel_role_access::Column::Deleted.eq(false))
                .add(
                    Condition::any()
                        .add(channel_role_access::Column::CanRead.eq(true))
                        .add(channel_role_access::Column::CanWrite.eq(true)),
                ),
        )
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?
        .into_iter()
        .map(|role_access| role_access.role_id)
        .collect();

    let admin_manage_role_ids: Vec<Uuid> = role::Entity::find()
        .filter(
            Condition::any()
                .add(role::Column::Administrator.eq(true))
                .add(role::Column::ManageChannels.eq(true)),
        )
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?
        .into_iter()
        .map(|role| role.id)
        .collect();

    let combined_role_ids: Vec<Uuid> = [role_ids, admin_manage_role_ids].concat();

    let user_role_access = user_role_access::Entity::find()
        .filter(user_role_access::Column::UserId.eq(user_id))
        .filter(user_role_access::Column::RoleId.is_in(combined_role_ids))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    Ok(user_role_access.is_some())
}

#[get("/search")]
pub async fn search_messages(
    app_state: web::Data<app_state::AppState>,
    req: HttpRequest,
    query: web::Query<HashMap<String, String>>,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req)?;

    let content = query.get("content").unwrap_or(&"".to_string()).to_string();

    let from_user_id = query
        .get("user_id")
        .unwrap_or(&Uuid::nil().to_string())
        .parse::<Uuid>()
        .unwrap_or(Uuid::nil());

    let recipient_type = query
        .get("recipient_type")
        .unwrap_or(&"".to_string())
        .to_string();

    let reference_id = query
        .get("reference_id")
        .unwrap_or(&Uuid::nil().to_string())
        .parse::<Uuid>()
        .unwrap_or(Uuid::nil());

    let mut query = message::Entity::find()
        .filter(message::Column::MessageType.eq("MESSAGE"))
        .filter(message::Column::Deleted.eq(false))
        .order_by_desc(message::Column::DateCreated);

    if !content.is_empty() {
        query = query.filter(message::Column::Content.contains(content));
    }

    if from_user_id != Uuid::nil() {
        query = query.filter(message::Column::UserId.eq(from_user_id));
    }

    if recipient_type == "USER" && from_user_id != Uuid::nil() {
        query = query.filter(message::Column::RecipientType.eq("USER"));
        query = query.filter(message::Column::ReferenceId.eq(user_id));
    } else if recipient_type == "CHANNEL" && reference_id != Uuid::nil() {
        query = query.filter(message::Column::RecipientType.eq("CHANNEL"));
        query = query.filter(message::Column::ReferenceId.eq(reference_id));

        let has_access_to_channel =
            get_user_has_access_to_channel(app_state.clone(), user_id, reference_id).await?;

        if !has_access_to_channel {
            return Err(ApiResponse::new(
                400,
                "User does not have access to channel".to_string(),
            ));
        }
    } else {
        return Err(ApiResponse::new(
            400,
            "Recipient type must be either USER or CHANNEL.".to_string(),
        ));
    }

    let messages = query
        .limit(10)
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    let response_dtos: Vec<MessageDTO> = messages.into_iter().map(MessageDTO::from).collect();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dtos).unwrap(),
    ))
}
