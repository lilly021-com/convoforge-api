use crate::utils::api_response::ApiResponse;
use crate::utils::app_state;
use crate::utils::chat::ChatRoom;
use crate::utils::jwt::get_user_id_from_http_request;
use crate::utils::logging::log_info;
use crate::utils::message::{
    send_update_status_from_channel_id, send_update_status_from_role_id_and_org_id,
};
use crate::utils::organization_util::get_organization_id_from_user_id;
use crate::utils::permissions::{check_permission, Permission};
use actix_web::{delete, get, patch, post, web, HttpRequest, Result};
use entity::{channel, channel_role_access, role, user, user_role_access};
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, JoinType, QueryFilter, QuerySelect, RelationTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct ChannelDTO {
    id: Option<Uuid>,
    name: String,
    description: Option<String>,
    deleted: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct ChannelIdDTO {
    id: Uuid,
}

impl From<channel::Model> for ChannelDTO {
    fn from(model: channel::Model) -> Self {
        Self {
            id: Some(model.id),
            name: model.name,
            description: model.description,
            deleted: Some(model.deleted),
        }
    }
}

#[post("/")]
pub async fn create_channel(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    channel_dto: web::Json<ChannelDTO>,
    chat_room: web::Data<Arc<ChatRoom>>,
) -> Result<ApiResponse, ApiResponse> {
    let has_manage_channels =
        check_permission(&app_state.db, req.clone(), Permission::ManageChannels).await;

    let organization_id = get_organization_id_from_user_id(
        &app_state.db,
        get_user_id_from_http_request(req.clone())?,
    )
    .await?;

    if !has_manage_channels {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to manage channels.".to_string(),
        ));
    }

    if channel_dto.name.is_empty() {
        return Err(ApiResponse::new(
            400,
            "Channel name must be at least 1 character.".to_string(),
        ));
    }

    let channel_model = channel::ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(channel_dto.name.clone()),
        description: Set(channel_dto.description.clone()),
        organization_id: Set(organization_id),
        ..Default::default()
    }
    .insert(&app_state.db)
    .await
    .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    log_info(req, format!("Created channel {}", channel_model.id));

    send_update_status_from_channel_id(channel_model.id, &app_state, &chat_room).await;

    let response_dto: ChannelDTO = channel_model.into();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dto).unwrap(),
    ))
}

#[get("/")]
async fn get_my(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req.clone())?;

    let has_manage_channels =
        check_permission(&app_state.db, req, Permission::ManageChannels).await;

    let user_organization_id = get_organization_id_from_user_id(&app_state.db, user_id).await?;

    if has_manage_channels {
        let channels = channel::Entity::find()
            .filter(channel::Column::Deleted.eq(false))
            .filter(channel::Column::OrganizationId.eq(user_organization_id))
            .all(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;
        let response_dtos: Vec<ChannelDTO> = channels.into_iter().map(ChannelDTO::from).collect();
        return Ok(ApiResponse::new(
            200,
            serde_json::to_string(&response_dtos).unwrap(),
        ));
    }

    let channels: Vec<channel::Model> = channel::Entity::find()
        .distinct()
        .join(JoinType::Join, channel::Relation::ChannelRoleAccess.def())
        .join(JoinType::Join, channel_role_access::Relation::Role.def())
        .join(JoinType::Join, role::Relation::UserRoleAccess.def())
        .join(JoinType::Join, user_role_access::Relation::User.def())
        .filter(channel::Column::Deleted.eq(false))
        .filter(channel_role_access::Column::Deleted.eq(false))
        .filter(user::Column::Id.eq(user_id))
        .filter(user::Column::OrganizationId.eq(user_organization_id))
        .filter(channel_role_access::Column::CanRead.eq(true))
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    let response_dtos: Vec<ChannelDTO> = channels.into_iter().map(ChannelDTO::from).collect();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dtos).unwrap(),
    ))
}

#[delete("/")]
pub async fn delete_channel(
    app_state: web::Data<app_state::AppState>,
    req: HttpRequest,
    channel_id_dto: web::Json<ChannelIdDTO>,
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

    let user_organization_id = get_organization_id_from_user_id(
        &app_state.db,
        get_user_id_from_http_request(req.clone())?,
    )
    .await?;

    let channel_model = channel::Entity::find()
        .filter(channel::Column::Id.eq(channel_id_dto.id))
        .filter(channel::Column::OrganizationId.eq(user_organization_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if channel_model.is_none() {
        return Err(ApiResponse::new(404, "Channel not found.".to_string()));
    }

    let channel_model = channel_model.unwrap();

    let user_id = get_user_id_from_http_request(req.clone())?;

    let user_organization_id = get_organization_id_from_user_id(&app_state.db, user_id).await?;

    let active_model = channel::ActiveModel {
        id: Set(channel_model.id),
        name: Set(channel_model.name.clone()),
        description: Set(channel_model.description.clone()),
        deleted: Set(true),
        organization_id: Set(user_organization_id),
    };

    active_model
        .update(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    let response_dto: ChannelDTO = channel_model.into();

    let channel_role_accesses = channel_role_access::Entity::find()
        .filter(channel_role_access::Column::ChannelId.eq(channel_id_dto.id))
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    for channel_role_access in channel_role_accesses {
        send_update_status_from_role_id_and_org_id(
            channel_role_access.role_id,
            user_organization_id,
            &app_state,
            &chat_room,
        )
        .await;
        let active_model = channel_role_access::ActiveModel {
            id: Set(channel_role_access.id),
            channel_id: Set(channel_role_access.channel_id),
            role_id: Set(channel_role_access.role_id),
            deleted: Set(true),
            ..Default::default()
        };
        active_model
            .update(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;
    }

    log_info(req, format!("Deleted channel {}", channel_id_dto.id));

    send_update_status_from_channel_id(channel_id_dto.id, &app_state, &chat_room).await;

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dto).unwrap(),
    ))
}

#[patch("/")]
pub async fn update_channel(
    app_state: web::Data<app_state::AppState>,
    req: HttpRequest,
    channel_dto: web::Json<ChannelDTO>,
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

    if channel_dto.name.is_empty() {
        return Err(ApiResponse::new(
            400,
            "Channel name must be at least 1 character.".to_string(),
        ));
    }

    let user_organization_id = get_organization_id_from_user_id(
        &app_state.db,
        get_user_id_from_http_request(req.clone())?,
    )
    .await?;

    let existing_channel = channel::Entity::find()
        .filter(channel::Column::Id.eq(channel_dto.id.unwrap()))
        .filter(channel::Column::Deleted.eq(false))
        .filter(channel::Column::OrganizationId.eq(user_organization_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if existing_channel.is_none() {
        return Err(ApiResponse::new(404, "Channel not found.".to_string()));
    }

    let channel_model = channel::ActiveModel {
        id: Set(channel_dto.id.unwrap()),
        name: Set(channel_dto.name.clone()),
        description: Set(channel_dto.description.clone()),
        organization_id: Set(user_organization_id),
        ..Default::default()
    };

    channel_model
        .update(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    log_info(req, format!("Updated channel {}", channel_dto.id.unwrap()));

    let response_dto = ChannelDTO {
        id: Some(channel_dto.id.unwrap()),
        name: channel_dto.name.clone(),
        description: channel_dto.description.clone(),
        deleted: None,
    };

    send_update_status_from_channel_id(channel_dto.id.unwrap(), &app_state, &chat_room).await;

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dto).unwrap(),
    ))
}
