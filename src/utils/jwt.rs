use super::constants;
use crate::utils::api_response::ApiResponse;
use actix_web::HttpRequest;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub struct Claims {
    pub exp: usize,
    pub iat: usize,
    pub username: String,
    pub id: Uuid,
}

pub fn encode_jwt(username: String, id: Uuid) -> Result<String, jsonwebtoken::errors::Error> {
    let now = Utc::now();
    let expire = Duration::hours(24);

    let claims: Claims = Claims {
        exp: (now + expire).timestamp() as usize,
        iat: now.timestamp() as usize,
        username,
        id,
    };

    let secret: String = (*constants::JWT_SECRET).clone();

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
}

pub fn decode_jwt(jwt: String) -> Result<TokenData<Claims>, jsonwebtoken::errors::Error> {
    let secret = (*constants::JWT_SECRET).clone();
    let claim_data: Result<TokenData<Claims>, jsonwebtoken::errors::Error> = decode(
        &jwt,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    );

    claim_data
}

// pub fn get_username_from_jwt(jwt: String) -> Result<String, jsonwebtoken::errors::Error> {
//     let claim_data = decode_jwt(jwt)?;
//     Ok(claim_data.claims.username)
// }

pub fn get_id_from_jwt(jwt: String) -> Result<Uuid, jsonwebtoken::errors::Error> {
    let claim_data = decode_jwt(jwt)?;
    Ok(claim_data.claims.id)
}

pub fn get_user_id_from_http_request(req: HttpRequest) -> Result<Uuid, ApiResponse> {
    // Extract token from headers
    let auth_header = req
        .headers()
        .get("Authorization")
        .ok_or(ApiResponse::new(401, "Unauthorized".to_string()))?
        .to_str()
        .map_err(|_| ApiResponse::new(401, "Unauthorized".to_string()))?;

    let token = auth_header.trim_start_matches("Bearer ").to_string();
    let user_id =
        get_id_from_jwt(token).map_err(|_| ApiResponse::new(401, "Unauthorized".to_string()))?;

    Ok(user_id)
}

pub fn get_user_id_from_token(token: String) -> Result<Uuid, ApiResponse> {
    let user_id =
        get_id_from_jwt(token).map_err(|_| ApiResponse::new(401, "Unauthorized".to_string()))?;

    Ok(user_id)
}

pub async fn get_client_secret_from_request(req: &HttpRequest) -> Result<String, ApiResponse> {
    req.headers()
        .get("Client-Secret")
        .and_then(|header| header.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| ApiResponse::new(400, "Invalid or missing Client-Secret header".to_string()))
}
