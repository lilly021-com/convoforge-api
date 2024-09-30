use crate::utils::jwt::get_client_secret_from_request;
use crate::utils::{api_response::ApiResponse, app_state, constants};
use actix_web::{put, web, HttpRequest};
use entity::flag;
use sea_orm::{
    entity::prelude::*, ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, Set,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct FlagDTO {
    pub name: String,
    pub enabled: bool,
}

impl From<flag::Model> for FlagDTO {
    fn from(flag: flag::Model) -> Self {
        Self {
            name: flag.name,
            enabled: flag.enabled,
        }
    }
}

#[put("/")]
pub async fn update_flag(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    flag_dto: web::Json<FlagDTO>,
) -> Result<ApiResponse, ApiResponse> {
    let client_secret = get_client_secret_from_request(&req).await?;

    if client_secret != *constants::CLIENT_SECRET {
        return Err(ApiResponse::new(
            401,
            "You are not authorized to perform this function".to_string(),
        ));
    }

    let flag_name = flag_dto.name.clone();
    let flag_enabled = flag_dto.enabled;

    let existing_flag = flag::Entity::find()
        .filter(flag::Column::Name.eq(&flag_name))
        .one(&app_state.db)
        .await
        .map_err(|_| ApiResponse::new(500, "Failed to retrieve flag".to_string()))?;

    if let Some(existing_flag) = existing_flag {
        let mut active_flag = existing_flag.into_active_model();
        active_flag.enabled = Set(flag_enabled);

        let updated_flag = active_flag
            .update(&app_state.db)
            .await
            .map_err(|_| ApiResponse::new(500, "Failed to update flag".to_string()))?;

        Ok(ApiResponse::new(
            200,
            serde_json::to_string(&FlagDTO::from(updated_flag)).unwrap(),
        ))
    } else {
        let new_flag = flag::ActiveModel {
            name: Set(flag_name),
            enabled: Set(flag_enabled),
            ..Default::default()
        };

        let created_flag = new_flag
            .insert(&app_state.db)
            .await
            .map_err(|_| ApiResponse::new(500, "Failed to create flag".to_string()))?;

        Ok(ApiResponse::new(
            201,
            serde_json::to_string(&FlagDTO::from(created_flag)).unwrap(),
        ))
    }
}
