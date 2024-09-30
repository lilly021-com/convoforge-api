use super::m20240802_093625_create_user_table::User;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserChannelView::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserChannelView::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserChannelView::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(UserChannelView::RecipientType)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserChannelView::ReferenceId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserChannelView::LastViewed)
                            .timestamp()
                            .not_null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-user_channel_view-user_id")
                            .from(UserChannelView::Table, UserChannelView::UserId)
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
                    .table(UserChannelView::Table)
                    .name("idx-user_channel_view-id")
                    .col(UserChannelView::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(UserChannelView::Table)
                    .name("idx-user_channel_view-user_id")
                    .col(UserChannelView::UserId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserChannelView::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UserChannelView {
    Table,
    Id,
    UserId,
    RecipientType,
    ReferenceId,
    LastViewed,
}
