use sea_orm_migration::prelude::*;

use super::m20240802_093625_create_user_table::User;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ProfileImageKey::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ProfileImageKey::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ProfileImageKey::UserId).uuid().not_null())
                    .col(ColumnDef::new(ProfileImageKey::Key).string().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-profile_image_key-user_id")
                            .from(ProfileImageKey::Table, ProfileImageKey::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(ProfileImageKey::Table)
                    .name("idx-profile_image_key-id")
                    .col(ProfileImageKey::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(ProfileImageKey::Table)
                    .name("idx-profile_image_key-user_id")
                    .col(ProfileImageKey::UserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ProfileImageKey::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ProfileImageKey {
    Table,
    Id,
    UserId,
    Key,
}
