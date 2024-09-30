use crate::utils::api_response::ApiResponse;
use crate::utils::app_state;
use crate::utils::jwt::get_user_id_from_http_request;
use crate::utils::organization_util::get_organization_id_from_user_id;
use crate::utils::permissions::{
    check_chat_permission, check_permission, ChatPermission, Permission,
};
use actix_web::{get, patch, web, HttpRequest, Result};
use entity::{
    channel, channel_role_access, message, role, user, user_channel_view, user_role_access,
};
use sea_orm::ActiveValue::Set;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, JoinType, QueryFilter, QueryOrder,
    QuerySelect, RelationTrait,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct ChannelViewDTO {
    recipient_type: String,
    reference_id: Uuid,
}

#[derive(Serialize, Deserialize)]
struct ResponseDTO {
    id: Uuid,
    last_viewed: chrono::NaiveDateTime,
    recipient_type: String,
    reference_id: Uuid,
    user_id: Uuid,
}

#[derive(Serialize, Deserialize)]
struct ChannelsDto {
    ids: Vec<ChannelViewDTO>,
}

#[derive(Serialize, Deserialize)]
struct ChannelDTO {
    id: Uuid,
    name: String,
    deleted: Option<bool>,
}

impl From<channel::Model> for ChannelDTO {
    fn from(model: channel::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            deleted: Some(model.deleted),
        }
    }
}

#[patch("/")]
pub async fn set_channel_last_viewed(
    app_state: web::Data<app_state::AppState>,
    req: HttpRequest,
    channel_id_dto: web::Json<ChannelViewDTO>,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req.clone())?;

    let channel_id = channel_id_dto.reference_id;
    let channel_type = channel_id_dto.recipient_type.clone();

    if channel_type == "CHANNEL" {
        let does_user_have_access = check_chat_permission(
            &app_state.db,
            req.clone(),
            ChatPermission::CanRead,
            channel_id,
        )
        .await;

        if !does_user_have_access {
            return Err(ApiResponse::new(
                403,
                "You do not have permission to view this channel.".to_string(),
            ));
        }
    }

    let user_channel_view = user_channel_view::Entity::find()
        .filter(user_channel_view::Column::UserId.eq(user_id))
        .filter(user_channel_view::Column::RecipientType.eq(channel_type.clone()))
        .filter(user_channel_view::Column::ReferenceId.eq(channel_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if user_channel_view.is_none() {
        let new_user_channel_view = user_channel_view::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(user_id),
            recipient_type: Set(channel_type),
            reference_id: Set(channel_id),
            last_viewed: Set(chrono::Utc::now().naive_utc()),
            ..Default::default()
        };

        new_user_channel_view
            .insert(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;
    } else {
        let user_channel_view = user_channel_view.unwrap();

        let mut user_channel_view = user_channel_view.into_active_model();
        user_channel_view.last_viewed = Set(chrono::Utc::now().naive_utc());
        user_channel_view.user_id = Set(user_id);
        user_channel_view.recipient_type = Set(channel_type);
        user_channel_view.reference_id = Set(channel_id);

        user_channel_view
            .update(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;
    }

    Ok(ApiResponse::new(
        200,
        "Channel last viewed updated".to_string(),
    ))
}

#[get("/unread")]
pub async fn get_unread_channels(
    app_state: web::Data<app_state::AppState>,
    req: HttpRequest,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req.clone())?;

    let user_organization_id = get_organization_id_from_user_id(&app_state.db, user_id).await?;

    // Check if the user has the ManageChannels permission
    let has_manage_channels =
        check_permission(&app_state.db, req, Permission::ManageChannels).await;

    // Fetch all channels or only those the user has access to
    let channels: Vec<channel::Model> = if has_manage_channels {
        channel::Entity::find()
            .filter(channel::Column::Deleted.eq(false))
            .filter(channel::Column::OrganizationId.eq(user_organization_id))
            .all(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?
    } else {
        channel::Entity::find()
            .distinct()
            .join(JoinType::Join, channel::Relation::ChannelRoleAccess.def())
            .join(JoinType::Join, channel_role_access::Relation::Role.def())
            .join(JoinType::Join, role::Relation::UserRoleAccess.def())
            .join(JoinType::Join, user_role_access::Relation::User.def())
            .filter(channel::Column::Deleted.eq(false))
            .filter(channel_role_access::Column::Deleted.eq(false))
            .filter(user_role_access::Column::Deleted.eq(false))
            .filter(channel_role_access::Column::CanRead.eq(true))
            .filter(user::Column::Id.eq(user_id))
            .all(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?
    };

    let mut unread_channels: Vec<ChannelViewDTO> = Vec::new();

    // Iterate over each channel to check for unread messages
    for channel in channels {
        // Find the most recent message in the channel
        let last_message = message::Entity::find()
            .filter(message::Column::ReferenceId.eq(channel.id))
            .filter(message::Column::RecipientType.eq("CHANNEL".to_string()))
            .order_by_desc(message::Column::DateCreated) // Order by most recent message
            .one(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;

        // Skip channels with no messages
        if last_message.is_none() {
            continue;
        }

        let user_channel_view = user_channel_view::Entity::find()
            .filter(user_channel_view::Column::UserId.eq(user_id))
            .filter(user_channel_view::Column::RecipientType.eq("CHANNEL".to_string()))
            .filter(user_channel_view::Column::ReferenceId.eq(channel.id))
            .one(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;

        // If no user_channel_view exists, mark the channel as unread
        if user_channel_view.is_none() {
            unread_channels.push(ChannelViewDTO {
                recipient_type: "CHANNEL".to_string(),
                reference_id: channel.id,
            });
        } else {
            let user_channel_view = user_channel_view.unwrap();
            let last_viewed = user_channel_view.last_viewed;

            // Compare the last message's timestamp with the last_viewed timestamp
            if last_message.unwrap().date_created > last_viewed {
                unread_channels.push(ChannelViewDTO {
                    recipient_type: "CHANNEL".to_string(),
                    reference_id: channel.id,
                });
            }
        }
    }

    // Fetch all users the current user has messaged with
    let users: Vec<user::Model> = user::Entity::find()
        .filter(user::Column::Id.ne(user_id)) // Exclude current user
        .filter(user::Column::OrganizationId.eq(user_organization_id))
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    // Iterate over each user to check for unread direct messages
    for user in users {
        // Find the most recent message from the user to the current user
        let last_message = message::Entity::find()
            .filter(message::Column::ReferenceId.eq(user_id))
            .filter(message::Column::RecipientType.eq("USER".to_string()))
            .filter(message::Column::UserId.eq(user.id))
            .order_by_desc(message::Column::DateCreated)
            .one(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;

        // Skip if there are no messages from this user
        if last_message.is_none() {
            continue;
        }

        let user_channel_view = user_channel_view::Entity::find()
            .filter(user_channel_view::Column::UserId.eq(user_id))
            .filter(user_channel_view::Column::RecipientType.eq("USER".to_string()))
            .filter(user_channel_view::Column::ReferenceId.eq(user.id))
            .one(&app_state.db)
            .await
            .map_err(|e| ApiResponse::new(500, e.to_string()))?;

        // If no user_channel_view exists, mark the message as unread
        if user_channel_view.is_none() {
            unread_channels.push(ChannelViewDTO {
                recipient_type: "USER".to_string(),
                reference_id: user.id,
            });
        } else {
            let user_channel_view = user_channel_view.unwrap();
            let last_viewed = user_channel_view.last_viewed;

            // Compare the last message's timestamp with the last_viewed timestamp
            if last_message.unwrap().date_created > last_viewed {
                unread_channels.push(ChannelViewDTO {
                    recipient_type: "USER".to_string(),
                    reference_id: user.id,
                });
            }
        }
    }

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&unread_channels).unwrap(),
    ))
}
