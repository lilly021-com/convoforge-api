use crate::routes::handlers::message_handler::get_user_has_access_to_channel;
use crate::utils::api_response::ApiResponse;
use crate::utils::app_state;
use crate::utils::jwt::get_user_id_from_http_request;
use crate::utils::logging::log_info;
use crate::utils::permissions::{check_permission, Permission};
use crate::utils::s3;
use actix_web::{delete, get, patch, post, web, HttpRequest};
use entity::media;
use sea_orm::prelude::DateTime;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(actix_multipart::form::MultipartForm)]
pub struct UploadMedia {
    #[multipart(limit = "1 MiB")]
    file: Option<actix_multipart::form::tempfile::TempFile>,
}

#[derive(Serialize, Deserialize)]
pub struct MediaAttachMessageDTO {
    media_id: Uuid,
    message_id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct MediaUploadDTO {
    id: Uuid,
    file_name: String,
    key: String,
    url: String,
    created_at: DateTime,
    user_id: Uuid,
    message_id: Option<Uuid>,
    deleted: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct MediaIdDTO {
    id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct MessageIdsDTO {
    ids: Vec<Uuid>,
}

#[post("/")]
pub async fn upload_media(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    form: actix_multipart::form::MultipartForm<UploadMedia>,
    s3_client: web::Data<s3::Client>,
) -> actix_web::Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req.clone())?;

    let file = form
        .file
        .as_ref()
        .ok_or(ApiResponse::new(400, "No file provided".to_string()))?;

    let key_prefix = format!("media/convoforge/{}/", s3::generate_random_session_id());

    let uploaded_file = s3_client.upload(file, &key_prefix).await;

    let uploaded_file = media::ActiveModel {
        id: Set(Uuid::new_v4()),
        file_name: Set(uploaded_file.filename),
        key: Set(uploaded_file.s3_key),
        url: Set(uploaded_file.s3_url),
        created_at: Set(chrono::Utc::now().naive_utc()),
        user_id: Set(user_id),
        ..Default::default()
    }
    .insert(&app_state.db)
    .await
    .unwrap();

    log_info(req, format!("Uploaded media {}", uploaded_file.id));

    let uploaded_file_dto = MediaUploadDTO {
        id: uploaded_file.id,
        file_name: uploaded_file.file_name,
        key: uploaded_file.key,
        url: uploaded_file.url,
        created_at: uploaded_file.created_at,
        user_id: uploaded_file.user_id,
        message_id: None,
        deleted: Some(uploaded_file.deleted),
    };

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&uploaded_file_dto).unwrap(),
    ))
}

#[patch("/")]
async fn attach_media_to_message(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    attach_message_dto: web::Json<MediaAttachMessageDTO>,
) -> actix_web::Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req.clone())?;

    let existing_media = media::Entity::find()
        .filter(media::Column::Id.eq(attach_message_dto.media_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if existing_media.is_none() {
        return Err(ApiResponse::new(404, "Media not found.".to_string()));
    }

    if existing_media.as_ref().unwrap().user_id != user_id {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to attach this media.".to_string(),
        ));
    }

    let mut media_model = existing_media.unwrap().into_active_model();
    media_model.message_id = Set(Some(attach_message_dto.message_id));

    let updated_media = media_model
        .update(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    log_info(
        req,
        format!(
            "Attached media {} to message {}",
            updated_media.id, attach_message_dto.message_id
        ),
    );

    let media_dto = MediaUploadDTO {
        id: updated_media.id,
        file_name: updated_media.file_name.clone(),
        key: updated_media.key.clone(),
        url: updated_media.url.clone(),
        created_at: updated_media.created_at,
        user_id: updated_media.user_id,
        message_id: updated_media.message_id,
        deleted: Some(updated_media.deleted),
    };

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&media_dto).unwrap(),
    ))
}

#[get("/")]
async fn get_all_media_by_message_id(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    query: web::Query<HashMap<String, String>>,
) -> actix_web::Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req)?;

    let message_id = query
        .get("message_id")
        .ok_or(ApiResponse::new(400, "Message ID is required".to_string()))?
        .parse::<Uuid>()
        .map_err(|e| ApiResponse::new(400, e.to_string()))?;

    let message = entity::message::Entity::find()
        .filter(entity::message::Column::Id.eq(message_id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?
        .unwrap();

    match message.recipient_type.as_str() {
        "USER" => {
            if message.user_id != user_id && message.reference_id != user_id {
                return Err(ApiResponse::new(
                    403,
                    "You do not have permission to view this message".to_string(),
                ));
            }
        }
        "CHANNEL" => {
            let has_user_channel_access =
                get_user_has_access_to_channel(app_state.clone(), user_id, message.reference_id)
                    .await
                    .unwrap();

            if !has_user_channel_access {
                return Err(ApiResponse::new(
                    403,
                    "You do not have permission to view this message".to_string(),
                ));
            }
        }
        _ => {
            return Err(ApiResponse::new(400, "Invalid recipient type".to_string()));
        }
    }

    let media = media::Entity::find()
        .filter(media::Column::MessageId.eq(message_id))
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    let media_dtos: Vec<MediaUploadDTO> = media
        .into_iter()
        .map(|m| MediaUploadDTO {
            id: m.id,
            file_name: m.file_name.clone(),
            key: m.key.clone(),
            url: m.url.clone(),
            created_at: m.created_at,
            user_id: m.user_id,
            message_id: Some(m.message_id.unwrap()),
            deleted: Some(m.deleted),
        })
        .collect();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&media_dtos).unwrap(),
    ))
}

#[delete("/")]
pub async fn delete_media(
    app_state: web::Data<app_state::AppState>,
    req: HttpRequest,
    media_id_dto: web::Json<MediaIdDTO>,
    s3_client: web::Data<s3::Client>,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req.clone())?;

    let media_model = media::Entity::find()
        .filter(media::Column::Id.eq(media_id_dto.id))
        .one(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    if media_model.is_none() {
        return Err(ApiResponse::new(404, "Media not found.".to_string()));
    }

    let is_admin = check_permission(&app_state.db, req.clone(), Permission::Administrator).await;

    if media_model.as_ref().unwrap().user_id != user_id && !is_admin {
        return Err(ApiResponse::new(
            403,
            "You do not have permission to delete this media.".to_string(),
        ));
    }

    let media_model = media_model.unwrap();

    s3_client.delete_file(&media_model.key).await;

    let active_model = media::ActiveModel {
        id: Set(media_model.id),
        file_name: Set(media_model.file_name.clone()),
        key: Set(media_model.key.clone()),
        url: Set(media_model.url.clone()),
        message_id: Set(media_model.message_id),
        created_at: Set(media_model.created_at),
        user_id: Set(media_model.user_id),
        deleted: Set(true),
    };

    active_model
        .update(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    log_info(req, format!("Deleted media {}", media_model.id));

    let response_dto = MediaUploadDTO {
        id: media_model.id,
        file_name: media_model.file_name.clone(),
        key: media_model.key.clone(),
        url: media_model.url.clone(),
        created_at: media_model.created_at,
        user_id: media_model.user_id,
        message_id: media_model.message_id,
        deleted: Some(true),
    };

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&response_dto).unwrap(),
    ))
}

#[post("/list")]
pub async fn get_media_by_list_of_message_ids(
    req: HttpRequest,
    app_state: web::Data<app_state::AppState>,
    message_ids_dto: web::Json<MessageIdsDTO>,
) -> Result<ApiResponse, ApiResponse> {
    let user_id = get_user_id_from_http_request(req)?;

    let messages = entity::message::Entity::find()
        .filter(entity::message::Column::Id.is_in(message_ids_dto.ids.clone()))
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    for message in messages {
        match message.recipient_type.as_str() {
            "USER" => {
                if message.user_id != user_id && message.reference_id != user_id {
                    return Err(ApiResponse::new(
                        403,
                        "You do not have permission to view this message".to_string(),
                    ));
                }
            }
            "CHANNEL" => {
                let has_user_channel_access = get_user_has_access_to_channel(
                    app_state.clone(),
                    user_id,
                    message.reference_id,
                )
                .await
                .unwrap();

                if !has_user_channel_access {
                    return Err(ApiResponse::new(
                        403,
                        "You do not have permission to view this message".to_string(),
                    ));
                }
            }
            _ => {
                return Err(ApiResponse::new(400, "Invalid recipient type".to_string()));
            }
        }
    }

    let media = media::Entity::find()
        .filter(media::Column::MessageId.is_in(message_ids_dto.ids.clone()))
        .all(&app_state.db)
        .await
        .map_err(|e| ApiResponse::new(500, e.to_string()))?;

    let media_dtos: Vec<MediaUploadDTO> = media
        .into_iter()
        .map(|m| MediaUploadDTO {
            id: m.id,
            file_name: m.file_name.clone(),
            key: m.key.clone(),
            url: m.url.clone(),
            created_at: m.created_at,
            user_id: m.user_id,
            message_id: Some(m.message_id.unwrap()),
            deleted: Some(m.deleted),
        })
        .collect();

    Ok(ApiResponse::new(
        200,
        serde_json::to_string(&media_dtos).unwrap(),
    ))
}
