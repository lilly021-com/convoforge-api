use crate::utils::constants;
use crate::utils::jwt::get_user_id_from_http_request;
use actix_web::HttpRequest;
use entity::{channel_role_access, role, user, user_role_access};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

#[derive(Clone, Copy)]
pub enum Permission {
    Administrator,
    ManageChannels,
    ManageRoles,
    ManageUsers,
}

#[derive(Clone, Copy)]
pub enum ChatPermission {
    CanRead,
    CanWrite,
}

pub async fn check_permission(
    db: &DatabaseConnection,
    req: HttpRequest,
    permission: Permission,
) -> bool {
    let has_client_secret = check_client_secret(req.clone());

    if has_client_secret {
        return true;
    }

    let user = match get_user_by_request(db, &req).await {
        Some(user) => user,
        None => return false,
    };

    let user_role_accesses = match get_user_role_accesses(db, user.id).await {
        Some(accesses) => accesses,
        None => return false,
    };

    for access in user_role_accesses {
        let role = match get_role_by_id(db, access.role_id).await {
            Some(role) => role,
            None => continue,
        };

        if role.administrator {
            return true;
        }

        if match_permission(permission, &role) {
            return true;
        }
    }

    false
}

pub async fn check_chat_permission(
    db: &DatabaseConnection,
    req: HttpRequest,
    permission: ChatPermission,
    channel_id: Uuid,
) -> bool {
    let user_id = get_user_id_from_http_request(req.clone()).unwrap();

    let user_organization_id = match user::Entity::find()
        .filter(user::Column::Id.eq(user_id))
        .one(db)
        .await
    {
        Ok(Some(user)) => user.organization_id,
        _ => return false,
    };

    let channel = match entity::channel::Entity::find()
        .filter(entity::channel::Column::Id.eq(channel_id))
        .one(db)
        .await
    {
        Ok(Some(channel)) => channel,
        _ => return false,
    };

    if channel.organization_id != user_organization_id {
        return false;
    }

    let has_client_secret = check_client_secret(req.clone());

    if has_client_secret {
        return true;
    }

    let user = match get_user_by_request(db, &req).await {
        Some(user) => user,
        None => return false,
    };

    let user_role_accesses = match get_user_role_accesses(db, user.id).await {
        Some(accesses) => accesses,
        None => return false,
    };

    for access in user_role_accesses {
        let role = match get_role_by_id(db, access.role_id).await {
            Some(role) => role,
            None => continue,
        };

        if role.administrator {
            return true;
        }

        let channel_role_access = match get_channel_role_access(db, channel_id, role.id).await {
            Some(access) => access,
            None => continue,
        };

        if match_chat_permission(permission, &channel_role_access) {
            return true;
        }
    }

    false
}

fn check_client_secret(req: HttpRequest) -> bool {
    let secret = (*constants::CLIENT_SECRET).clone();

    if let Some(client_secret) = req.headers().get("Client-Secret") {
        if let Ok(client_secret) = client_secret.to_str() {
            if client_secret == secret {
                return true;
            }
        }
    }

    false
}

async fn get_user_by_request(db: &DatabaseConnection, req: &HttpRequest) -> Option<user::Model> {
    let user_id = get_user_id_from_http_request(req.clone()).unwrap();

    match user::Entity::find()
        .filter(user::Column::Id.eq(user_id))
        .one(db)
        .await
    {
        Ok(Some(user)) => Some(user),
        _ => None,
    }
}

async fn get_user_role_accesses(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Option<Vec<user_role_access::Model>> {
    match user_role_access::Entity::find()
        .filter(user_role_access::Column::UserId.eq(user_id))
        .all(db)
        .await
    {
        Ok(accesses) => Some(accesses),
        _ => None,
    }
}

async fn get_role_by_id(db: &DatabaseConnection, role_id: Uuid) -> Option<role::Model> {
    match role::Entity::find()
        .filter(role::Column::Id.eq(role_id))
        .one(db)
        .await
    {
        Ok(Some(role)) => Some(role),
        _ => None,
    }
}

async fn get_channel_role_access(
    db: &DatabaseConnection,
    channel_id: Uuid,
    role_id: Uuid,
) -> Option<channel_role_access::Model> {
    match channel_role_access::Entity::find()
        .filter(channel_role_access::Column::ChannelId.eq(channel_id))
        .filter(channel_role_access::Column::RoleId.eq(role_id))
        .one(db)
        .await
    {
        Ok(Some(access)) => Some(access),
        _ => None,
    }
}

fn match_permission(permission: Permission, role: &role::Model) -> bool {
    match permission {
        Permission::Administrator => role.administrator,
        Permission::ManageChannels => role.manage_channels,
        Permission::ManageRoles => role.manage_roles,
        Permission::ManageUsers => role.manage_users,
    }
}

fn match_chat_permission(
    permission: ChatPermission,
    channel_role_access: &channel_role_access::Model,
) -> bool {
    match permission {
        ChatPermission::CanRead => channel_role_access.can_read || channel_role_access.can_write,
        ChatPermission::CanWrite => channel_role_access.can_write,
    }
}
