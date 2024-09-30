use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use dotenv::dotenv;
use log::info;
use migration::{Migrator, MigratorTrait};
use sea_orm::Database;
use std::sync::Arc;
use utils::app_state::AppState;
use utils::chat::ChatRoom;
use utils::s3::configure_and_return_s3_client;

pub mod middlewares;
mod routes;
mod utils;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let database_connection_url = (*utils::constants::DATABASE_URL).clone();
    let args: Vec<String> = std::env::args().collect();

    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info,sqlx=warn"));

    info!("Starting ConvoForge Server...");

    let db = Database::connect(database_connection_url).await.unwrap();
    Migrator::up(&db, None).await.unwrap();

    if args.len() > 1 && args[1] == "seed" {
        utils::seed::seed_data(&db).await;
    }

    let chat_room = Arc::new(ChatRoom::new());

    let s3_client = web::Data::new(configure_and_return_s3_client().await);

    HttpServer::new(move || {
        App::new()
            .wrap(
                Cors::permissive()
                    .allow_any_origin()
                    .allow_any_method()
                    .allow_any_header()
                    .max_age(3600),
            )
            .app_data(web::Data::new(AppState { db: db.clone() }))
            .app_data(web::Data::new(chat_room.clone()))
            .app_data(s3_client.clone())
            .configure(routes::auth_routes::config)
            .configure(routes::channel_routes::config)
            .configure(routes::role_routes::config)
            .configure(routes::user_role_access_routes::config)
            .configure(routes::channel_role_access_routes::config)
            .configure(routes::message_routes::config)
            .configure(routes::chat_routes::config)
            .configure(routes::presence_routes::config)
            .configure(routes::media_routes::config)
            .configure(routes::user_routes::config)
            .configure(routes::user_channel_view_routes::config)
            .configure(routes::seen_message_routes::config)
            .configure(routes::organization_routes::config)
            .configure(routes::flag_routes::config)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
