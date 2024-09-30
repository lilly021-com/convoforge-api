use crate::utils;
use crate::utils::api_response::ApiResponse;
use crate::utils::chat::ChatRoom;
use crate::utils::jwt::get_user_id_from_http_request;
use crate::utils::logging::log_info;
use crate::utils::message::send_update_status_to_all_users;
use crate::utils::organization_util::get_organization_id_from_user_id;
use crate::utils::permissions::{
    check_chat_permission, check_permission, ChatPermission, Permission,
};
use crate::utils::s3::generate_random_session_id;
use crate::utils::{app_state, s3};
use actix_web::{delete, get, patch, web, HttpRequest, Result};
use entity::{channel, channel_role_access, role, user, user_role_access};
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, JoinType, PaginatorTrait,
    QueryFilter, QuerySelect, RelationTrait,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use utils::key_update;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct UserDTO {
    id: Uuid,
    username: String,
    display_name: Option<String>,
    profile_image: Option<String>,
}

#[derive(actix_multipart::form::MultipartForm)]
pub struct UploadPicture {
    #[multipart(limit = "1 MiB")]
    file: Option<actix_multipart::form::tempfile::TempFile>,
}

impl From<user::Model> for UserDTO {
    fn from(model: user::Model) -> Self {
        Self {
            id: model.id,
            username: model.username,
            display_name: Some(model.display_name),
            profile_image: model.profile_image,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct UserResponseDTO {
    id: Uuid,
    username: String,
    display_name: String,
}

#[derive(Serialize, Deserialize)]
struct UserUpdateDTO {
    id: Uuid,
    display_name: String,
}

#[get("/")]
pub async fn get_users_by_page(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    query: web::Query<HashMap<String, String>>,
) -> Result<ApiResponse, ApiResponse> {
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

    let user_id = get_user_id_from_http_request(req.clone())?;

    let user_organization_id = get_organization_id_from_user_id(&app_state.db, user_id).await?;

    let query = user::Entity::find()
        .filter(user::Column::Deleted.eq(false))
        .filter(user::Column::OrganizationId.eq(user_organization_id));

    let paginator = query.paginate(&app_state.db, per_page);

    let users = paginator
        .fetch_page(page - 1)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    let users_dto: Vec<UserDTO> = users.into_iter().map(|user| user.into()).collect();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&users_dto).unwrap(),
    ))
}

#[delete("/purge")]
pub async fn purge_user(
    query: web::Query<HashMap<String, String>>,
    app_state: web::Data<app_state::AppState>,
    req: HttpRequest,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = query
        .get("user_id")
        .unwrap_or(&Uuid::nil().to_string())
        .parse::<Uuid>()
        .unwrap_or(Uuid::nil());

    let is_admin = check_permission(&app_state.db, req.clone(), Permission::Administrator).await;

    let current_user_id = get_user_id_from_http_request(req.clone())?;

    let user_organization_id =
        get_organization_id_from_user_id(&app_state.db, current_user_id).await?;

    if !is_admin {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to manage roles.".to_string(),
        ));
    }

    let user = user::Entity::find()
        .filter(user::Column::Id.eq(user_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if user.is_none() {
        return Err(ApiResponse::new(404, "User not found".to_string()));
    }

    let user = user.unwrap();

    if user.organization_id != user_organization_id {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to manage this user".to_string(),
        ));
    }

    let user_role_accesses = user_role_access::Entity::find()
        .filter(user_role_access::Column::UserId.eq(user.id))
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

    //set all messages to be deleted
    let messages = entity::message::Entity::find()
        .filter(entity::message::Column::UserId.eq(user.id))
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    for message in messages {
        let mut message = message.into_active_model();
        message.deleted = Set(true);
        message
            .update(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;
    }

    let mut user = user.into_active_model();
    user.deleted = Set(true);
    user.update(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    log_info(req, format!("Purged user {}", user_id));

    Ok(ApiResponse::new(200, "User purged".to_string()))
}

#[get("/channel/{channel_id}")]
pub async fn get_users_in_channel(
    app_state: web::Data<app_state::AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req.clone())?;

    let user_organization_id = get_organization_id_from_user_id(&app_state.db, user_id).await?;

    let channel_id = path.parse::<Uuid>().unwrap_or(Uuid::nil());

    let has_channel_access =
        check_chat_permission(&app_state.db, req, ChatPermission::CanRead, channel_id).await;

    if !has_channel_access {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to view users in this channel.".to_string(),
        ));
    }

    let channel = entity::channel::Entity::find()
        .filter(channel::Column::Id.eq(channel_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if channel.is_none() {
        return Err(ApiResponse::new(404, "Channel not found".to_string()));
    }

    let channel = channel.unwrap();

    if channel.organization_id != user_organization_id {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to view users in this channel.".to_string(),
        ));
    }

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

    let paginator = user::Entity::find()
        .join(JoinType::Join, user::Relation::UserRoleAccess.def())
        .join(JoinType::Join, user_role_access::Relation::Role.def())
        .join(JoinType::Join, role::Relation::ChannelRoleAccess.def())
        .join(JoinType::Join, channel_role_access::Relation::Channel.def())
        .filter(channel::Column::Deleted.eq(false))
        .filter(user_role_access::Column::Deleted.eq(false))
        .filter(channel_role_access::Column::Deleted.eq(false))
        .filter(user::Column::Deleted.eq(false))
        .filter(channel_role_access::Column::CanRead.eq(true))
        .filter(channel::Column::Id.eq(channel_id))
        .paginate(&app_state.db, per_page);

    let users = paginator
        .fetch_page(page - 1)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    let users_dto: Vec<UserDTO> = users.into_iter().map(|user| user.into()).collect();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&users_dto).unwrap(),
    ))
}

#[get("/current")]
pub async fn get_current_user(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req)?;

    let user = user::Entity::find()
        .filter(user::Column::Id.eq(user_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if user.is_none() {
        return Err(ApiResponse::new(404, "User not found".to_string()));
    }

    let user = user.unwrap();

    let user_dto: UserDTO = user.into();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&user_dto).unwrap(),
    ))
}

#[patch("/name")]
pub async fn update_display_name(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    user_update_dto: web::Json<UserUpdateDTO>,
    chat_room: web::Data<Arc<ChatRoom>>,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req.clone())?;

    let user_organization_id = get_organization_id_from_user_id(&app_state.db, user_id).await?;

    let has_manage_users =
        check_permission(&app_state.db, req.clone(), Permission::ManageUsers).await;

    if !has_manage_users && user_id != user_update_dto.id {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to update this user's display name".to_string(),
        ));
    }

    if user_update_dto.display_name.is_empty() {
        return Err(ApiResponse::new(
            400,
            "Display name must be greater than 0 characters".to_string(),
        ));
    }

    let existing_user = user::Entity::find()
        .filter(user::Column::DisplayName.eq(user_update_dto.display_name.clone()))
        .filter(user::Column::OrganizationId.eq(user_organization_id))
        .filter(user::Column::Id.ne(user_update_dto.id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if existing_user.is_some() {
        return Err(ApiResponse::new(
            409,
            "Display name already in use".to_string(),
        ));
    }

    let user = user::Entity::find()
        .filter(user::Column::Id.eq(user_update_dto.id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if let Some(user) = user {
        //check to see if the organizations match
        let user_organization_id = get_organization_id_from_user_id(&app_state.db, user_id).await?;

        if user.organization_id != user_organization_id {
            return Err(ApiResponse::new(
                403,
                "You do not have permission to update this user's display name".to_string(),
            ));
        }

        let mut active_user = user.into_active_model();
        active_user.display_name = Set(user_update_dto.display_name.clone());

        let updated_user = active_user
            .update(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;

        let user_response_dto = UserResponseDTO {
            id: updated_user.id,
            username: updated_user.username.clone(),
            display_name: updated_user.display_name.clone(),
        };

        send_update_status_to_all_users(user_organization_id, &app_state, &chat_room).await;

        Ok(ApiResponse::new(
            200,
            serde_json::to_string(&user_response_dto).unwrap(),
        ))
    } else {
        Err(ApiResponse::new(404, "User not found".to_string()))
    }
}

#[patch("/profile-image")]
pub async fn update_profile_image(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    form: actix_multipart::form::MultipartForm<UploadPicture>,
    s3_client: web::Data<s3::Client>,
    chat_room: web::Data<Arc<ChatRoom>>,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req.clone())?;

    let file = form
        .file
        .as_ref()
        .ok_or(ApiResponse::new(400, "No file provided".to_string()))?;

    //make sure the file is an image otherwise return error
    let content_type = file.content_type.clone();
    if let Some(content_type) = content_type {
        if !content_type.to_string().starts_with("image/") {
            return Err(ApiResponse::new(400, "File must be an image".to_string()));
        }
    } else {
        return Err(ApiResponse::new(
            400,
            "File type could not be determined".to_string(),
        ));
    }

    let key_prefix = format!(
        "media/convoforge/profile-user-{}/{}/",
        user_id,
        generate_random_session_id()
    );

    let profile_key = key_update::fetch_profile_key(user_id, &app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    let uploaded_file = s3_client.upload(file, &key_prefix).await;

    key_update::update_profile_key(user_id, uploaded_file.s3_key, &app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if let Some(profile_key) = profile_key {
        s3_client.delete_file(&profile_key).await;
    }

    let image_url = uploaded_file.s3_url.clone();

    let user = user::Entity::find()
        .filter(user::Column::Id.eq(user_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if let Some(user) = user {
        let mut active_user = user.into_active_model();
        active_user.profile_image = Set(Some(image_url));

        let user_organization_id = get_organization_id_from_user_id(&app_state.db, user_id).await?;

        active_user
            .update(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;

        send_update_status_to_all_users(user_organization_id, &app_state, &chat_room).await;

        Ok(ApiResponse::new(
            200,
            "User profile image updated".to_string(),
        ))
    } else {
        Err(ApiResponse::new(404, "User not found".to_string()))
    }
}
