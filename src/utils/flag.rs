use crate::utils::app_state;
use actix_web::web::Data;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub async fn is_flag_on(flag_name: &str, app_state: &Data<app_state::AppState>) -> bool {
    let flag = entity::flag::Entity::find()
        .filter(entity::flag::Column::Name.eq(flag_name))
        .one(&app_state.db)
        .await
        .unwrap();

    if let Some(flag) = flag {
        return flag.enabled;
    }

    false
}
