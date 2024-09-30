use super::handlers;
use actix_web::web;

pub fn config(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/organization")
            .service(handlers::organization_handler::get_organization_exists)
            .service(handlers::organization_handler::get_all_organizations),
    );
}
