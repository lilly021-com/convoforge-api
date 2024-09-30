use sea_orm_migration::{prelude::*, schema::*};

use super::m20240802_093625_create_user_table::User;
use super::m20240805_132555_create_message_table::Message;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Media::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Media::Id).uuid().not_null().primary_key())
                    .col(string(Media::FileName).not_null())
                    .col(string(Media::Key).not_null())
                    .col(string(Media::Url).not_null())
                    .col(ColumnDef::new(Media::MessageId).uuid())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-media-message_id")
                            .from(Media::Table, Media::MessageId)
                            .to(Message::Table, Message::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(ColumnDef::new(Media::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Media::UserId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-media-user_id")
                            .from(Media::Table, Media::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(Media::Deleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Media::Table)
                    .name("idx-media-id")
                    .col(Media::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Media::Table)
                    .name("idx-media-message_id")
                    .col(Media::MessageId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Media::Table)
                    .name("idx-media-user_id")
                    .col(Media::UserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Media::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Media {
    Table,
    Id,
    FileName,
    Key,
    Url,
    MessageId,
    UserId,
    CreatedAt,
    Deleted,
}
