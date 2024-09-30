use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Flag::Table)
                    .if_not_exists()
                    .col(string(Flag::Name).not_null().primary_key())
                    .col(boolean(Flag::Enabled).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Flag::Table)
                    .name("idx-flag-name")
                    .col(Flag::Name)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Flag::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum Flag {
    Table,
    Name,
    Enabled,
}
