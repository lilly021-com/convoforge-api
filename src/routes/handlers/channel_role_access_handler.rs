use crate::utils::api_response::ApiResponse;
use crate::utils::app_state;
use crate::utils::chat::ChatRoom;
use crate::utils::jwt::get_user_id_from_http_request;
use crate::utils::logging::log_info;
use crate::utils::message::send_update_status_from_role_id_and_org_id;
use crate::utils::organization_util::get_organization_id_from_user_id;
use crate::utils::permissions::{check_permission, Permission};
use actix_web::{delete, get, post, web, HttpRequest, Result};
use entity::channel_role_access;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct ChannelRoleAccessDTO {
    id: Option<Uuid>,
    channel_id: Uuid,
    role_id: Uuid,
    deleted: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct ChannelRoleAccessIdDTO {
    id: Uuid,
}

impl From<channel_role_access::Model> for ChannelRoleAccessDTO {
    fn from(model: channel_role_access::Model) -> Self {
        Self {
            id: Some(model.id),
            channel_id: model.channel_id,
            role_id: model.role_id,
            deleted: Some(model.deleted),
        }
    }
}

async fn find_channel_role_accesses_by_channel_id(
    db: &sea_orm::DatabaseConnection,
    channel_id: Uuid,
) -> Result<Vec<channel_role_access::Model>, ApiResponse> {
    channel_role_access::Entity::find()
        .filter(channel_role_access::Column::ChannelId.eq(channel_id))
        .filter(channel_role_access::Column::Deleted.eq(false))
        .all(db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))
}

#[post("/")]
pub async fn create_channel_role_access(
    app_state: web::Data<app_state::AppState>,
    req: HttpRequest,
    channel_role_access_dto: web::Json<ChannelRoleAccessDTO>,
    chat_room: web::Data<Arc<ChatRoom>>,
) -> Result<ApiResponse, ApiResponse> {
    let has_manage_channels =
        check_permission(&app_state.db, req.clone(), Permission::ManageChannels).await;

    let user_id = get_user_id_from_http_request(req.clone())?;

    let user_organization_id = get_organization_id_from_user_id(&app_state.db, user_id).await?;

    if !has_manage_channels {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to manage channels.".to_string(),
        ));
    }

    //check to make sure the channel associated with the channel role access is in the same organization as the user
    let channel = entity::channel::Entity::find()
        .filter(entity::channel::Column::Id.eq(channel_role_access_dto.channel_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if channel.is_none() {
        return Err(ApiResponse::new(404, "Channel not found.".to_string()));
    }

    let channel = channel.unwrap();

    if channel.organization_id != user_organization_id {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to manage channels in this organization.".to_string(),
        ));
    }

    let channel_role_access_model = channel_role_access::ActiveModel {
        id: Set(Uuid::new_v4()),
        channel_id: Set(channel_role_access_dto.channel_id),
        role_id: Set(channel_role_access_dto.role_id),
        ..Default::default()
    }
    .insert(&app_state.db)
    .await
    .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    log_info(
        req,
        format!(
            "Created channel role access {} with channel Id {} and role {}",
            channel_role_access_model.id,
            channel_role_access_model.channel_id,
            channel_role_access_model.role_id
        ),
    );

    let response_dto: ChannelRoleAccessDTO = channel_role_access_model.into();

    send_update_status_from_role_id_and_org_id(
        response_dto.role_id,
        user_organization_id,
        &app_state,
        &chat_room,
    )
    .await;

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dto).unwrap(),
    ))
}

#[get("/")]
async fn get_all(
    app_state: web::Data<app_state::AppState>,
    req: HttpRequest,
    query: web::Query<HashMap<String, String>>,
) -> Result<ApiResponse, ApiResponse> {
    let has_manage_channels =
        check_permission(&app_state.db, req, Permission::ManageChannels).await;

    if !has_manage_channels {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to manage channels.".to_string(),
        ));
    }

    let channel_id = query
        .get("channel_id")
        .unwrap_or(&Uuid::nil().to_string())
        .parse::<Uuid>()
        .unwrap_or(Uuid::nil());

    let channel_role_access = if channel_id == Uuid::nil() {
        channel_role_access::Entity::find()
            .filter(channel_role_access::Column::Deleted.eq(false))
            .all(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?
    } else {
        find_channel_role_accesses_by_channel_id(&app_state.db, channel_id).await?
    };

    let response_dtos: Vec<ChannelRoleAccessDTO> = channel_role_access
        .into_iter()
        .map(ChannelRoleAccessDTO::from)
        .collect();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dtos).unwrap(),
    ))
}

#[delete("/")]
pub async fn delete(
    app_state: web::Data<app_state::AppState>,
    req: HttpRequest,
    channel_role_access_id: web::Json<ChannelRoleAccessIdDTO>,
    chat_room: web::Data<Arc<ChatRoom>>,
) -> Result<ApiResponse, ApiResponse> {
    let has_manage_channels =
        check_permission(&app_state.db, req.clone(), Permission::ManageChannels).await;

    if !has_manage_channels {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to manage channels.".to_string(),
        ));
    }

    let user_id = get_user_id_from_http_request(req.clone())?;

    let user_organization_id = get_organization_id_from_user_id(&app_state.db, user_id).await?;

    let channel_role_access_model = channel_role_access::Entity::find()
        .filter(channel_role_access::Column::Id.eq(channel_role_access_id.id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if channel_role_access_model.is_none() {
        return Err(ApiResponse::new(
            404,
            "Channel role access not found".to_string(),
        ));
    }

    let channel_role_access_model = channel_role_access_model.unwrap();

    let channel = entity::channel::Entity::find()
        .filter(entity::channel::Column::Id.eq(channel_role_access_model.channel_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if channel.is_none() {
        return Err(ApiResponse::new(404, "Channel not found.".to_string()));
    }

    let channel = channel.unwrap();

    if channel.organization_id != user_organization_id {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to manage channels in this organization.".to_string(),
        ));
    }

    let active_model = channel_role_access::ActiveModel {
        id: Set(channel_role_access_model.id),
        channel_id: Set(channel_role_access_model.channel_id),
        role_id: Set(channel_role_access_model.role_id),
        deleted: Set(true),
        ..Default::default()
    };

    active_model
        .update(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    log_info(
        req,
        format!(
            "Deleted channel role access {} with channel Id {} and role {}",
            channel_role_access_model.id,
            channel_role_access_model.channel_id,
            channel_role_access_model.role_id
        ),
    );

    let response_dto: ChannelRoleAccessDTO = channel_role_access_model.into();

    send_update_status_from_role_id_and_org_id(
        response_dto.role_id,
        user_organization_id,
        &app_state,
        &chat_room,
    )
    .await;

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dto).unwrap(),
    ))
}
