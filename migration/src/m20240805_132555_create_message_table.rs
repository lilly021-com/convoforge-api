use sea_orm_migration::{prelude::*, schema::*};

use super::m20240802_093625_create_user_table::User;
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Message::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Message::Id).uuid().not_null().primary_key())
                    .col(uuid(Message::UserId).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-message-user_id")
                            .from(Message::Table, Message::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(Message::Deleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Message::Content).string())
                    .col(ColumnDef::new(Message::DateCreated).timestamp().not_null())
                    .col(ColumnDef::new(Message::DateUpdated).timestamp().not_null())
                    .col(ColumnDef::new(Message::MessageType).string().not_null())
                    .col(ColumnDef::new(Message::RecipientType).string().not_null())
                    .col(ColumnDef::new(Message::ReferenceId).uuid().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Message::Table)
                    .name("idx-message-id")
                    .col(Message::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Message::Table)
                    .name("idx-message-user_id")
                    .col(Message::UserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Message::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Message {
    Table,
    Id,
    UserId,
    Content,
    DateCreated,
    DateUpdated,
    MessageType,
    RecipientType,
    ReferenceId,
    Deleted,
}
