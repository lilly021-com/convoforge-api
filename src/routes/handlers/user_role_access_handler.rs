use crate::utils::api_response::ApiResponse;
use crate::utils::app_state;
use crate::utils::chat::ChatRoom;
use crate::utils::jwt::get_user_id_from_http_request;
use crate::utils::logging::log_info;
use crate::utils::message::send_update_status_from_role_id_and_org_id;
use crate::utils::organization_util::get_organization_id_from_user_id;
use crate::utils::permissions::{check_permission, Permission};
use actix_web::{delete, get, post, web, HttpRequest, Result};
use entity::user_role_access;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct UserRoleAccessDTO {
    id: Option<Uuid>,
    user_id: Uuid,
    role_id: Uuid,
    deleted: Option<bool>,
}

#[derive(Serialize, Deserialize)]
struct UserRoleAccessIdDTO {
    id: Uuid,
}

#[derive(Serialize, Deserialize)]
struct MessageDTO {
    message_type: String,
}

impl From<user_role_access::Model> for UserRoleAccessDTO {
    fn from(model: user_role_access::Model) -> Self {
        Self {
            id: Some(model.id),
            user_id: model.user_id,
            role_id: model.role_id,
            deleted: Some(model.deleted),
        }
    }
}

#[post("/")]
pub async fn create_user_role_access(
    app_state: web::Data<app_state::AppState>,
    user_role_access_dto: web::Json<UserRoleAccessDTO>,
    req: HttpRequest,
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

    //check to see if the user is in the same organization as the user they are trying to assign a role to
    let user_organization_id_to_assign =
        get_organization_id_from_user_id(&app_state.db, user_role_access_dto.user_id).await?;

    if user_organization_id != user_organization_id_to_assign {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to assign roles to users outside of your organization."
                .to_string(),
        ));
    }

    //check to see if the role id is part of the current organization

    let role = entity::role::Entity::find()
        .filter(entity::role::Column::Id.eq(user_role_access_dto.role_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if role.is_none() {
        return Err(ApiResponse::new(404, "Role not found.".to_string()));
    }

    let role = role.unwrap();

    if role.organization_id != user_organization_id {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to assign roles outside of your organization.".to_string(),
        ));
    }

    let user_role_access_model = user_role_access::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_role_access_dto.user_id),
        role_id: Set(user_role_access_dto.role_id),
        ..Default::default()
    }
    .insert(&app_state.db)
    .await
    .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    log_info(
        req,
        format!(
            "User {} was assigned role {}",
            user_role_access_dto.user_id, user_role_access_dto.role_id
        ),
    );

    let response_dto: UserRoleAccessDTO = user_role_access_model.into();

    send_update_status_from_role_id_and_org_id(
        user_role_access_dto.role_id,
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
    query: web::Query<HashMap<String, String>>,
) -> Result<ApiResponse, ApiResponse> {
    let mut ura_query =
        user_role_access::Entity::find().filter(user_role_access::Column::Deleted.eq(false));

    let user_id = query
        .get("user_id")
        .unwrap_or(&Uuid::nil().to_string())
        .parse::<Uuid>()
        .unwrap_or(Uuid::nil());

    if user_id != Uuid::nil() {
        ura_query = ura_query.filter(user_role_access::Column::UserId.eq(user_id));
    }

    let user_role_access = ura_query
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    let response_dtos: Vec<UserRoleAccessDTO> = user_role_access
        .into_iter()
        .map(UserRoleAccessDTO::from)
        .collect();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dtos).unwrap(),
    ))
}

#[delete("/")]
pub async fn delete(
    app_state: web::Data<app_state::AppState>,
    user_role_access_dto: web::Json<UserRoleAccessIdDTO>,
    req: HttpRequest,
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

    let user_role_access_model = user_role_access::Entity::find()
        .filter(user_role_access::Column::Id.eq(user_role_access_dto.id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if user_role_access_model.is_none() {
        return Err(ApiResponse::new(
            404,
            "User role access not found.".to_string(),
        ));
    }

    let user_role_access_model = user_role_access_model.unwrap();

    let role = entity::role::Entity::find()
        .filter(entity::role::Column::Id.eq(user_role_access_model.role_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if role.is_none() {
        return Err(ApiResponse::new(404, "Role not found.".to_string()));
    }

    let role = role.unwrap();

    if role.organization_id != user_organization_id {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to delete roles outside of your organization.".to_string(),
        ));
    }

    let mut user_role_access_model = user_role_access_model.into_active_model();
    user_role_access_model.deleted = Set(true);

    let updated_user_role_access = user_role_access_model
        .update(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    log_info(
        req,
        format!("User role access {} was deleted", user_role_access_dto.id),
    );

    send_update_status_from_role_id_and_org_id(
        updated_user_role_access.role_id,
        user_organization_id,
        &app_state,
        &chat_room,
    )
    .await;

    let response_dto: UserRoleAccessDTO = updated_user_role_access.into();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dto).unwrap(),
    ))
}
