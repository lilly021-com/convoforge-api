use chrono::Utc;
use entity::{channel, channel_role_access, message, organization, role, user, user_role_access};
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use uuid::Uuid;

pub async fn seed_data(db: &DatabaseConnection) {
    for org_index in 1..=100 {
        let org_id = Uuid::new_v4();

        // Create organization
        organization::ActiveModel {
            id: Set(org_id),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();

        // Create role
        let role_id = Uuid::new_v4();
        role::ActiveModel {
            id: Set(role_id),
            name: Set("Administrator".to_string()),
            administrator: Set(true),
            manage_channels: Set(true),
            organization_id: Set(org_id),
            ..Default::default()
        }
        .insert(db)
        .await
        .unwrap();

        let mut user_ids = Vec::with_capacity(100);
        let mut channel_ids = Vec::with_capacity(100);

        // Create users
        for user_index in 1..=100 {
            let user_id = Uuid::new_v4();
            user_ids.push(user_id);
            user::ActiveModel {
                id: Set(user_id),
                username: Set(format!("user_{}_{}", org_index, user_index)),
                display_name: Set(format!("User {}_{}", org_index, user_index)),
                organization_id: Set(org_id),
                ..Default::default()
            }
            .insert(db)
            .await
            .unwrap();
        }

        // Assign role to users
        for user_id in &user_ids {
            user_role_access::ActiveModel {
                id: Set(Uuid::new_v4()),
                user_id: Set(*user_id),
                role_id: Set(role_id),
                deleted: Set(false),
            }
            .insert(db)
            .await
            .unwrap();
        }

        // Create channels
        for channel_index in 1..=10 {
            let channel_id = Uuid::new_v4();
            channel_ids.push(channel_id);
            channel::ActiveModel {
                id: Set(channel_id),
                name: Set(format!("channel_{}_{}", org_index, channel_index)),
                organization_id: Set(org_id),
                ..Default::default()
            }
            .insert(db)
            .await
            .unwrap();

            // Assign role to channel
            channel_role_access::ActiveModel {
                id: Set(Uuid::new_v4()),
                can_read: Set(true),
                can_write: Set(true),
                channel_id: Set(channel_id),
                role_id: Set(role_id),
                deleted: Set(false),
            }
            .insert(db)
            .await
            .unwrap();
        }

        // Create messages for each user and channel
        for user_id in &user_ids {
            for channel_id in &channel_ids {
                for _ in 1..=10 {
                    message::ActiveModel {
                        id: Set(Uuid::new_v4()),
                        reference_id: Set(*channel_id),
                        recipient_type: Set("CHANNEL".to_string()),
                        content: Set(Some(format!(
                            "Test message from user_{}_{} in channel_{}_{}",
                            org_index, user_id, org_index, channel_id
                        ))),
                        user_id: Set(*user_id),
                        date_updated: Set(Utc::now().naive_utc()),
                        date_created: Set(Utc::now().naive_utc()),
                        message_type: Set("MESSAGE".to_string()),
                        deleted: Set(false),
                    }
                    .insert(db)
                    .await
                    .unwrap();
                }
            }
        }
    }
}
