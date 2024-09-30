use crate::utils::api_response::ApiResponse;
use crate::utils::app_state::AppState;
use crate::utils::flag::is_flag_on;
use actix_web::body::MessageBody;
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::{web, Error};
use actix_web_lab::middleware::Next;

pub async fn get_s3_flag_on(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let app_state = req
        .app_data::<web::Data<AppState>>()
        .ok_or_else(|| Error::from(ApiResponse::new(500, "App state not available".to_string())))?;

    let s3_flag = is_flag_on("s3", &app_state).await;

    if s3_flag {
        next.call(req).await
    } else {
        Err(Error::from(ApiResponse::new(
            403,
            "S3 is disabled".to_string(),
        )))
    }
}
