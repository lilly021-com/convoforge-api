use sea_orm_migration::prelude::*;

use super::m20240801_133022_create_organization_table::Organization;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(User::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(User::Username).string().not_null())
                    .col(ColumnDef::new(User::DisplayName).string().not_null())
                    .col(ColumnDef::new(User::ProfileImage).string())
                    .col(ColumnDef::new(User::OrganizationId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-user-organization_id")
                            .from(User::Table, User::OrganizationId)
                            .to(Organization::Table, Organization::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(
                        ColumnDef::new(User::Deleted)
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
                    .table(User::Table)
                    .name("idx-user-username")
                    .col(User::Username)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(User::Table)
                    .name("idx-user-id")
                    .col(User::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(User::Table)
                    .name("idx-user-organization_id")
                    .col(User::OrganizationId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(User::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum User {
    Table,
    Id,
    Username,
    DisplayName,
    ProfileImage,
    Deleted,
    OrganizationId,
}
