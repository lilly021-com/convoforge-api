use crate::utils::api_response::ApiResponse;
use entity::user;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

pub async fn get_organization_id_from_user_id(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Uuid, ApiResponse> {
    let user = user::Entity::find()
        .filter(user::Column::Id.eq(user_id))
        .one(db)
        .await
        .map_err(|_| ApiResponse::new(500, "Failed to get user".to_string()))?;

    if user.is_none() {
        return Err(ApiResponse::new(404, "User not found".to_string()));
    }

    let user = user.unwrap();

    Ok(user.organization_id)
}
