use crate::utils::api_response::ApiResponse;
use crate::utils::app_state;
use crate::utils::chat::ChatRoom;
use crate::utils::jwt::get_user_id_from_http_request;
use crate::utils::logging::log_info;
use crate::utils::message::send_update_status_from_role_id_and_org_id;
use crate::utils::organization_util::get_organization_id_from_user_id;
use crate::utils::permissions::{check_permission, Permission};
use actix_web::{delete, get, patch, post, web, HttpRequest, Result};
use entity::role;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct RoleDTO {
    id: Option<Uuid>,
    name: String,
    administrator: bool,
    manage_users: bool,
    manage_channels: bool,
    manage_roles: bool,
    deleted: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct RoleIdDTO {
    id: Uuid,
}

#[derive(Serialize, Deserialize)]
struct RoleResponseDTO {
    id: Option<Uuid>,
    name: String,
    administrator: bool,
    manage_users: bool,
    manage_channels: bool,
    manage_roles: bool,
    deleted: bool,
}

#[derive(Serialize, Deserialize)]
struct MessageDTO {
    message_type: String,
}

impl From<role::Model> for RoleDTO {
    fn from(model: role::Model) -> Self {
        Self {
            id: Some(model.id),
            name: model.name,
            administrator: model.administrator,
            manage_users: model.manage_users,
            manage_channels: model.manage_channels,
            manage_roles: model.manage_roles,
            deleted: Some(model.deleted),
        }
    }
}

#[post("/")]
async fn create_role(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    role_dto: web::Json<RoleDTO>,
) -> Result<ApiResponse, ApiResponse> {
    let has_manage_roles =
        check_permission(&app_state.db, req.clone(), Permission::ManageRoles).await;

    let user_id = get_user_id_from_http_request(req.clone())?;

    let user_organization_id = get_organization_id_from_user_id(&app_state.db, user_id).await?;

    if !has_manage_roles {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to manage roles.".to_string(),
        ));
    }

    if role_dto.name.is_empty() {
        return Err(ApiResponse::new(
            400,
            "Role name must be at least 1 character.".to_string(),
        ));
    }

    let role_model = role::ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(role_dto.name.clone()),
        administrator: Set(role_dto.administrator),
        manage_users: Set(role_dto.manage_users),
        manage_channels: Set(role_dto.manage_channels),
        manage_roles: Set(role_dto.manage_roles),
        organization_id: Set(user_organization_id),
        ..Default::default()
    }
    .insert(&app_state.db)
    .await
    .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    log_info(req, format!("Created role {}", role_model.id));

    let response_dto: RoleDTO = role_model.into();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dto).unwrap(),
    ))
}

#[get("/")]
async fn get_all(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    query: web::Query<HashMap<String, String>>,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = query
        .get("user_id")
        .unwrap_or(&Uuid::nil().to_string())
        .parse::<Uuid>()
        .unwrap_or(Uuid::nil());

    let current_user_id = get_user_id_from_http_request(req.clone())?;

    let user_organization_id =
        get_organization_id_from_user_id(&app_state.db, current_user_id).await?;

    //if user_id is 0 then return all roles, otherwise get all roles belonging to user_id
    let roles = if user_id == Uuid::nil() {
        role::Entity::find()
            .filter(role::Column::Deleted.eq(false))
            .filter(role::Column::OrganizationId.eq(user_organization_id))
            .all(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?
    } else {
        let user_role_accesses = entity::user_role_access::Entity::find()
            .filter(entity::user_role_access::Column::UserId.eq(user_id))
            .all(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;

        let role_ids: Vec<Uuid> = user_role_accesses
            .into_iter()
            .map(|ura| ura.role_id)
            .collect();

        role::Entity::find()
            .filter(role::Column::Id.is_in(role_ids))
            .filter(role::Column::Deleted.eq(false))
            .filter(role::Column::OrganizationId.eq(user_organization_id))
            .all(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?
    };

    let response_dtos: Vec<RoleDTO> = roles.into_iter().map(RoleDTO::from).collect();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dtos).unwrap(),
    ))
}

#[patch("/")]
async fn update_role(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    role_dto: web::Json<RoleDTO>,
    chat_room: web::Data<Arc<ChatRoom>>,
) -> Result<ApiResponse, ApiResponse> {
    let has_manage_roles =
        check_permission(&app_state.db, req.clone(), Permission::ManageRoles).await;

    if !has_manage_roles {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to manage roles.".to_string(),
        ));
    }

    let user_id = get_user_id_from_http_request(req.clone())?;

    let user_organization_id = get_organization_id_from_user_id(&app_state.db, user_id).await?;

    let existing_role = role::Entity::find()
        .filter(role::Column::Id.eq(role_dto.id.unwrap()))
        .filter(role::Column::Deleted.eq(false))
        .filter(role::Column::OrganizationId.eq(user_organization_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if existing_role.is_none() {
        return Err(ApiResponse::new(404, "Role not found.".to_string()));
    }

    if role_dto.name.is_empty() {
        return Err(ApiResponse::new(
            400,
            "Role name must be at least 1 character.".to_string(),
        ));
    }

    let role_model = role::ActiveModel {
        id: Set(role_dto.id.unwrap()),
        name: Set(role_dto.name.clone()),
        administrator: Set(role_dto.administrator),
        manage_users: Set(role_dto.manage_users),
        manage_channels: Set(role_dto.manage_channels),
        manage_roles: Set(role_dto.manage_roles),
        organization_id: Set(user_organization_id),
        ..Default::default()
    };

    role_model
        .update(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    log_info(req, format!("Updated role {}", role_dto.id.unwrap()));

    let response_dto = RoleResponseDTO {
        id: Some(role_dto.id.unwrap()),
        name: role_dto.name.clone(),
        administrator: role_dto.administrator,
        manage_users: role_dto.manage_users,
        manage_channels: role_dto.manage_channels,
        manage_roles: role_dto.manage_roles,
        deleted: false,
    };

    send_update_status_from_role_id_and_org_id(
        role_dto.id.unwrap(),
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

#[delete("/")]
async fn delete_role(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    role_dto: web::Json<RoleIdDTO>,
    chat_room: web::Data<Arc<ChatRoom>>,
) -> Result<ApiResponse, ApiResponse> {
    let has_manage_roles =
        check_permission(&app_state.db, req.clone(), Permission::ManageRoles).await;

    if !has_manage_roles {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to manage roles.".to_string(),
        ));
    }

    let user_id = get_user_id_from_http_request(req.clone())?;

    let user_organization_id = get_organization_id_from_user_id(&app_state.db, user_id).await?;

    let existing_role = role::Entity::find()
        .filter(role::Column::Id.eq(role_dto.id))
        .filter(role::Column::Deleted.eq(false))
        .filter(role::Column::OrganizationId.eq(user_organization_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if existing_role.is_none() {
        return Err(ApiResponse::new(404, "Role not found.".to_string()));
    }

    let role_model = role::ActiveModel {
        id: Set(role_dto.id),
        deleted: Set(true),
        organization_id: Set(user_organization_id),
        ..Default::default()
    };

    role_model
        .update(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    log_info(req, format!("Deleted role {}", role_dto.id));

    let response_dto = RoleResponseDTO {
        id: Some(role_dto.id),
        name: "".to_string(),
        administrator: false,
        manage_users: false,
        manage_channels: false,
        manage_roles: false,
        deleted: true,
    };

    //get all user role accesses with this role id and set them to deleted
    let user_role_accesses = entity::user_role_access::Entity::find()
        .filter(entity::user_role_access::Column::RoleId.eq(role_dto.id))
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    for user_role_access in user_role_accesses {
        let mut user_role_access = user_role_access.into_active_model();
        user_role_access.deleted = Set(true);
        user_role_access
            .update(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;
    }

    send_update_status_from_role_id_and_org_id(
        role_dto.id,
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
