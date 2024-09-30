use std::env;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref DATABASE_URL: String = set_database_url();
    pub static ref CLIENT_SECRET: String = set_client_secret();
    pub static ref JWT_SECRET: String = set_jwt_secret();
    pub static ref AWS_S3_BUCKET_NAME: String = set_aws_s3_bucket_name();
    pub static ref AWS_REGION: String = set_aws_region();
    pub static ref AWS_ACCESS_KEY_ID: String = set_aws_access_key_id();
    pub static ref AWS_SECRET_ACCESS_KEY: String = set_aws_secret_access_key();
}

fn set_database_url() -> String {
    dotenv::dotenv().ok();
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}

fn set_client_secret() -> String {
    dotenv::dotenv().ok();
    env::var("CLIENT_SECRET").expect("CLIENT_SECRET must be set")
}

fn set_jwt_secret() -> String {
    dotenv::dotenv().ok();
    env::var("JWT_SECRET").expect("JWT_SECRET must be set")
}

fn set_aws_s3_bucket_name() -> String {
    dotenv::dotenv().ok();
    env::var("AWS_S3_BUCKET_NAME").expect("AWS_S3_BUCKET_NAME must be set")
}

fn set_aws_region() -> String {
    dotenv::dotenv().ok();
    env::var("AWS_REGION").expect("AWS_REGION must be set")
}

fn set_aws_access_key_id() -> String {
    dotenv::dotenv().ok();
    env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID must be set")
}

fn set_aws_secret_access_key() -> String {
    dotenv::dotenv().ok();
    env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_ACCESS_KEY must be set")
}
