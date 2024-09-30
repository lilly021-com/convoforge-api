use sea_orm_migration::{prelude::*, schema::*};

use super::m20240805_080851_create_channel_table::Channel;
use super::m20240805_095452_create_role_table::Role;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ChannelRoleAccess::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ChannelRoleAccess::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(boolean(ChannelRoleAccess::CanRead).not_null().default(true))
                    .col(
                        boolean(ChannelRoleAccess::CanWrite)
                            .not_null()
                            .default(true),
                    )
                    .col(uuid(ChannelRoleAccess::ChannelId).not_null())
                    .col(uuid(ChannelRoleAccess::RoleId).not_null())
                    .col(
                        ColumnDef::new(ChannelRoleAccess::Deleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-channelroleaccess-channel_id")
                            .from(ChannelRoleAccess::Table, ChannelRoleAccess::ChannelId)
                            .to(Channel::Table, Channel::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-channelroleaccess-role_id")
                            .from(ChannelRoleAccess::Table, ChannelRoleAccess::RoleId)
                            .to(Role::Table, Role::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(ChannelRoleAccess::Table)
                    .name("idx-channelroleaccess-id")
                    .col(ChannelRoleAccess::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(ChannelRoleAccess::Table)
                    .name("idx-channelroleaccess-channel_id")
                    .col(ChannelRoleAccess::ChannelId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(ChannelRoleAccess::Table)
                    .name("idx-channelroleaccess-role_id")
                    .col(ChannelRoleAccess::RoleId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ChannelRoleAccess::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ChannelRoleAccess {
    Table,
    Id,
    CanRead,
    CanWrite,
    ChannelId,
    RoleId,
    Deleted,
}
