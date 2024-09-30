use crate::utils::app_state;
use crate::utils::chat::ChatRoom;
use actix_web::web::Data;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct MessageDTO {
    message_type: String,
}

pub async fn send_update_status_from_role_id_and_org_id(
    role_id: Uuid,
    organization_id: Uuid,
    app_state: &Data<app_state::AppState>,
    chat_room: &Data<Arc<ChatRoom>>,
) {
    let user_role_accesses = entity::user_role_access::Entity::find()
        .filter(entity::user_role_access::Column::RoleId.eq(role_id))
        .all(&app_state.db)
        .await
        .unwrap();

    let mut user_ids: HashSet<Uuid> = HashSet::new();

    for user_role_access in user_role_accesses {
        user_ids.insert(user_role_access.user_id);
    }

    let admin_and_manage_roles = entity::role::Entity::find()
        .filter(
            entity::role::Column::Administrator
                .eq(true)
                .or(entity::role::Column::ManageRoles.eq(true)),
        )
        .filter(entity::role::Column::OrganizationId.eq(organization_id))
        .all(&app_state.db)
        .await
        .unwrap();

    let admin_and_manage_role_ids: Vec<Uuid> = admin_and_manage_roles
        .into_iter()
        .map(|role| role.id)
        .collect();

    let admin_and_manage_role_user_role_accesses = entity::user_role_access::Entity::find()
        .filter(entity::user_role_access::Column::RoleId.is_in(admin_and_manage_role_ids))
        .all(&app_state.db)
        .await
        .unwrap();

    for user_role_access in admin_and_manage_role_user_role_accesses {
        user_ids.insert(user_role_access.user_id);
    }

    let update_message_dto = MessageDTO {
        message_type: "UPDATE_STATUS".to_string(),
    };

    chat_room.send_message(
        &user_ids.into_iter().collect::<Vec<Uuid>>(),
        &serde_json::to_string(&update_message_dto).unwrap(),
    );
}

pub async fn send_update_status_from_channel_id(
    channel_id: Uuid,
    app_state: &Data<app_state::AppState>,
    chat_room: &Data<Arc<ChatRoom>>,
) {
    let channel_role_accesses = entity::channel_role_access::Entity::find()
        .filter(entity::channel_role_access::Column::ChannelId.eq(channel_id))
        .filter(entity::channel_role_access::Column::Deleted.eq(false))
        .all(&app_state.db)
        .await
        .unwrap();

    let mut user_ids: HashSet<Uuid> = HashSet::new();

    for channel_role_access in channel_role_accesses {
        let user_role_accesses = entity::user_role_access::Entity::find()
            .filter(entity::user_role_access::Column::RoleId.eq(channel_role_access.role_id))
            .filter(entity::user_role_access::Column::Deleted.eq(false))
            .all(&app_state.db)
            .await
            .unwrap();

        for user_role_access in user_role_accesses {
            user_ids.insert(user_role_access.user_id);
        }
    }

    let admin_and_manage_channel_roles = entity::role::Entity::find()
        .filter(
            entity::role::Column::Administrator
                .eq(true)
                .or(entity::role::Column::ManageChannels.eq(true)),
        )
        .all(&app_state.db)
        .await
        .unwrap();

    let admin_and_manage_channel_role_ids: Vec<Uuid> = admin_and_manage_channel_roles
        .into_iter()
        .map(|role| role.id)
        .collect();

    let admin_and_manage_channel_user_role_accesses = entity::user_role_access::Entity::find()
        .filter(entity::user_role_access::Column::RoleId.is_in(admin_and_manage_channel_role_ids))
        .filter(entity::user_role_access::Column::Deleted.eq(false))
        .all(&app_state.db)
        .await
        .unwrap();

    for user_role_access in admin_and_manage_channel_user_role_accesses {
        user_ids.insert(user_role_access.user_id);
    }

    let update_message_dto = MessageDTO {
        message_type: "UPDATE_STATUS".to_string(),
    };

    chat_room.send_message(
        &user_ids.into_iter().collect::<Vec<Uuid>>(),
        &serde_json::to_string(&update_message_dto).unwrap(),
    );
}

pub async fn send_update_status_to_all_users(
    organization_id: Uuid,
    app_state: &Data<app_state::AppState>,
    chat_room: &Data<Arc<ChatRoom>>,
) {
    let users = entity::user::Entity::find()
        .filter(entity::user::Column::Deleted.eq(false))
        .filter(entity::user::Column::OrganizationId.eq(organization_id))
        .all(&app_state.db)
        .await
        .unwrap();

    let user_ids: Vec<Uuid> = users.into_iter().map(|user| user.id).collect();

    let update_message_dto = MessageDTO {
        message_type: "UPDATE_STATUS".to_string(),
    };

    chat_room.send_message(
        &user_ids.into_iter().collect::<Vec<Uuid>>(),
        &serde_json::to_string(&update_message_dto).unwrap(),
    );
}
