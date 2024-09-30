use crate::utils::api_response::ApiResponse;
use crate::utils::jwt::get_client_secret_from_request;
use crate::utils::{app_state, constants};
use actix_web::{get, web, HttpRequest};
use entity::organization;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct OrganizationDTO {
    id: Uuid,
}

impl From<organization::Model> for OrganizationDTO {
    fn from(model: organization::Model) -> Self {
        Self { id: model.id }
    }
}

#[get("/")]
pub async fn get_organization_exists(
    app_state: web::Data<app_state::AppState>,
    query: web::Query<HashMap<String, String>>,
) -> Result<ApiResponse, ApiResponse> {
    let slug = query.get("slug").unwrap_or(&"".to_string()).to_string();

    let slug = Uuid::parse_str(&slug).map_err(|e| ApiResponse::new(400, e.to_string()))?;

    let organization = organization::Entity::find()
        .filter(organization::Column::Id.eq(slug))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if organization.is_none() {
        return Ok(ApiResponse::new(404, "Not found".to_string()));
    }

    Ok(ApiResponse::new(200, "Organization exists".to_string()))
}

#[get("/all")]
pub async fn get_all_organizations(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
) -> Result<ApiResponse, ApiResponse> {
    let client_secret = get_client_secret_from_request(&req).await?;

    if client_secret != *constants::CLIENT_SECRET {
        return Err(ApiResponse::new(404, "Not found".to_string()));
    }

    let organizations = organization::Entity::find()
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    let response_dtos: Vec<OrganizationDTO> = organizations
        .into_iter()
        .map(OrganizationDTO::from)
        .collect();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dtos).unwrap(),
    ))
}
