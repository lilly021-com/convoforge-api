use super::m20240802_093625_create_user_table::User;
use super::m20240805_132555_create_message_table::Message;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(SeenMessage::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SeenMessage::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(SeenMessage::UserId).uuid().not_null())
                    .col(ColumnDef::new(SeenMessage::MessageId).uuid().not_null())
                    .col(ColumnDef::new(SeenMessage::DateSeen).timestamp().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-seen_message-user_id")
                            .from(SeenMessage::Table, SeenMessage::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-seen_message-message_id")
                            .from(SeenMessage::Table, SeenMessage::MessageId)
                            .to(Message::Table, Message::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(SeenMessage::Table)
                    .name("idx-seen_message-id")
                    .col(SeenMessage::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(SeenMessage::Table)
                    .name("idx-seen_message-user_id")
                    .col(SeenMessage::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(SeenMessage::Table)
                    .name("idx-seen_message-message_id")
                    .col(SeenMessage::MessageId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(SeenMessage::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum SeenMessage {
    Table,
    Id,
    UserId,
    MessageId,
    DateSeen,
}
