use entity::profile_image_key;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set,
};
use uuid::Uuid;

pub async fn update_profile_key(
    user_id: Uuid,
    key: String,
    db: &DatabaseConnection,
) -> Result<(), sea_orm::DbErr> {
    let existing_key = profile_image_key::Entity::find()
        .filter(profile_image_key::Column::UserId.eq(user_id))
        .one(db)
        .await?;

    if let Some(existing_key) = existing_key {
        let mut active_model = existing_key.into_active_model();
        active_model.key = Set(key);
        active_model.update(db).await?;
    } else {
        let new_key = profile_image_key::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            key: Set(key),
            ..Default::default()
        };
        new_key.insert(db).await?;
    }

    Ok(())
}

pub async fn fetch_profile_key(
    user_id: Uuid,
    db: &DatabaseConnection,
) -> Result<Option<String>, sea_orm::DbErr> {
    // Find the profile image key for the user
    let profile_key = profile_image_key::Entity::find()
        .filter(profile_image_key::Column::UserId.eq(user_id))
        .one(db)
        .await?;

    // Return the key if it exists, otherwise return None
    Ok(profile_key.map(|key| key.key))
}
