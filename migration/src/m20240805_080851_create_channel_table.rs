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
                    .table(Channel::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Channel::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(Channel::Name).string().not_null())
                    .col(ColumnDef::new(Channel::Description).string())
                    .col(
                        ColumnDef::new(Channel::Deleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .col(ColumnDef::new(Channel::OrganizationId).uuid().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-channel-organization_id")
                            .from(Channel::Table, Channel::OrganizationId)
                            .to(Organization::Table, Organization::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Channel::Table)
                    .name("idx-channel-id")
                    .col(Channel::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(Channel::Table)
                    .name("idx-channel-organization_id")
                    .col(Channel::OrganizationId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Channel::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub(crate) enum Channel {
    Table,
    Id,
    Name,
    Description,
    Deleted,
    OrganizationId,
}
