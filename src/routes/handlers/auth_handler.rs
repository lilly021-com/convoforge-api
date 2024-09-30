use crate::utils::jwt::get_client_secret_from_request;
use crate::utils::{api_response::ApiResponse, app_state, constants, jwt::encode_jwt};
use actix_web::{post, web, HttpRequest};
use entity::{organization, user};
use sea_orm::{
    entity::prelude::*, ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, Set,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Structs and DTOs
#[derive(Serialize, Deserialize)]
struct SecretModel {
    username: String,
    display_name: String,
    slug: Uuid,
}

#[derive(Serialize)]
struct TokenResponse {
    token: String,
}

#[derive(Serialize)]
struct SlugResponse {
    id: Uuid,
}

#[post("/secret")]
pub async fn secret(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    secret_json: web::Json<SecretModel>,
) -> Result<ApiResponse, ApiResponse> {
    let secret = get_client_secret_from_request(&req).await?;

    if secret != *constants::CLIENT_SECRET {
        return Err(ApiResponse::new(404, "Not found".to_string()));
    }

    let org_id = secret_json.slug;

    let organization = organization::Entity::find()
        .filter(organization::Column::Id.eq(org_id))
        .one(&app_state.db)
        .await
        .map_err(|_| ApiResponse::new(500, "Database error: organization retrieval".to_string()))?
        .ok_or_else(|| ApiResponse::new(404, "Organization not found".to_string()))?;

    let mut user_data = get_or_create_user(
        &app_state.db,
        &secret_json.username,
        &secret_json.display_name,
        organization.id,
    )
    .await?;

    // Re-enable user if previously deleted
    if user_data.deleted {
        user_data = reactivate_user(&app_state.db, user_data).await?;
    }

    let token = encode_jwt(user_data.username, user_data.id)
        .map_err(|_| ApiResponse::new(500, "Token encoding error".to_string()))?;

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&TokenResponse { token }).unwrap(),
    ))
}

#[post("/organization")]
pub async fn create_organization(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
) -> Result<ApiResponse, ApiResponse> {
    let client_secret = get_client_secret_from_request(&req).await?;

    if client_secret != *constants::CLIENT_SECRET {
        return Err(ApiResponse::new(404, "Not found".to_string()));
    }

    let new_organization = organization::ActiveModel {
        id: Set(Uuid::new_v4()),
    }
    .insert(&app_state.db)
    .await
    .map_err(|_| ApiResponse::new(500, "Failed to create organization".to_string()))?;

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&SlugResponse {
            id: new_organization.id,
        })
        .unwrap(),
    ))
}

async fn get_or_create_user(
    db: &DatabaseConnection,
    username: &str,
    display_name: &str,
    organization_id: Uuid,
) -> Result<user::Model, ApiResponse> {
    if let Some(user) = get_user_by_username_and_org_id(db, username, organization_id).await? {
        return Ok(user);
    }

    let new_user = user::ActiveModel {
        id: Set(Uuid::new_v4()),
        username: Set(username.to_string()),
        display_name: Set(display_name.to_string()),
        organization_id: Set(organization_id),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|_| ApiResponse::new(500, "Database error: creating user".to_string()))?;

    Ok(new_user)
}

async fn reactivate_user(
    db: &DatabaseConnection,
    user: user::Model,
) -> Result<user::Model, ApiResponse> {
    let mut active_user = user.into_active_model();
    active_user.deleted = Set(false);

    active_user
        .update(db)
        .await
        .map_err(|_| ApiResponse::new(500, "Database error: reactivating user".to_string()))
}

async fn get_user_by_username_and_org_id(
    db: &DatabaseConnection,
    username: &str,
    organization_id: Uuid,
) -> Result<Option<user::Model>, ApiResponse> {
    user::Entity::find()
        .filter(user::Column::Username.eq(username))
        .filter(user::Column::OrganizationId.eq(organization_id))
        .one(db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))
}
