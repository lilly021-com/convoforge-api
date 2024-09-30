use sea_orm_migration::{prelude::*, schema::*};

use super::m20240802_093625_create_user_table::User;
use super::m20240805_095452_create_role_table::Role;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(UserRoleAccess::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(UserRoleAccess::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(uuid(UserRoleAccess::UserId).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-userroleaccess-user_id")
                            .from(UserRoleAccess::Table, UserRoleAccess::UserId)
                            .to(User::Table, User::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .col(uuid(UserRoleAccess::RoleId).not_null())
                    .col(
                        ColumnDef::new(UserRoleAccess::Deleted)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk-userroleaccess-role_id")
                            .from(UserRoleAccess::Table, UserRoleAccess::RoleId)
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
                    .table(UserRoleAccess::Table)
                    .name("idx-userroleaccess-id")
                    .col(UserRoleAccess::Id)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(UserRoleAccess::Table)
                    .name("idx-userroleaccess-user_id")
                    .col(UserRoleAccess::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .table(UserRoleAccess::Table)
                    .name("idx-userroleaccess-role_id")
                    .col(UserRoleAccess::RoleId)
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(UserRoleAccess::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum UserRoleAccess {
    Table,
    Id,
    UserId,
    RoleId,
    Deleted,
}
