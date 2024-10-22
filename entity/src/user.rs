//! `SeaORM` Entity, @generated by sea-orm-codegen 1.0.0

use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "user")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub username: String,
    pub display_name: String,
    pub profile_image: Option<String>,
    pub organization_id: Uuid,
    pub deleted: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::media::Entity")]
    Media,
    #[sea_orm(has_many = "super::message::Entity")]
    Message,
    #[sea_orm(
        belongs_to = "super::organization::Entity",
        from = "Column::OrganizationId",
        to = "super::organization::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Organization,
    #[sea_orm(has_many = "super::profile_image_key::Entity")]
    ProfileImageKey,
    #[sea_orm(has_many = "super::seen_message::Entity")]
    SeenMessage,
    #[sea_orm(has_many = "super::user_channel_view::Entity")]
    UserChannelView,
    #[sea_orm(has_many = "super::user_role_access::Entity")]
    UserRoleAccess,
}

impl Related<super::media::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Media.def()
    }
}

impl Related<super::message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Message.def()
    }
}

impl Related<super::organization::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organization.def()
    }
}

impl Related<super::profile_image_key::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProfileImageKey.def()
    }
}

impl Related<super::seen_message::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SeenMessage.def()
    }
}

impl Related<super::user_channel_view::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserChannelView.def()
    }
}

impl Related<super::user_role_access::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::UserRoleAccess.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
