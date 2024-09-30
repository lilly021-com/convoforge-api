pub use sea_orm_migration::prelude::*;
mod m20240801_133022_create_organization_table;
mod m20240802_093625_create_user_table;
mod m20240805_080851_create_channel_table;
mod m20240805_095452_create_role_table;
mod m20240805_104642_create_channel_role_access_table;
mod m20240805_112029_create_user_role_access_table;
mod m20240805_132555_create_message_table;
mod m20240808_074828_create_media_table;
mod m20240827_121544_create_user_channel_view_table;
mod m20240829_110635_create_seen_message_table;
mod m20240830_125050_create_profile_image_key_table;
mod m20240924_101830_create_flag_table;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240801_133022_create_organization_table::Migration),
            Box::new(m20240802_093625_create_user_table::Migration),
            Box::new(m20240805_080851_create_channel_table::Migration),
            Box::new(m20240805_095452_create_role_table::Migration),
            Box::new(m20240805_104642_create_channel_role_access_table::Migration),
            Box::new(m20240805_112029_create_user_role_access_table::Migration),
            Box::new(m20240805_132555_create_message_table::Migration),
            Box::new(m20240808_074828_create_media_table::Migration),
            Box::new(m20240827_121544_create_user_channel_view_table::Migration),
            Box::new(m20240829_110635_create_seen_message_table::Migration),
            Box::new(m20240830_125050_create_profile_image_key_table::Migration),
            Box::new(m20240924_101830_create_flag_table::Migration),
        ]
    }
}
