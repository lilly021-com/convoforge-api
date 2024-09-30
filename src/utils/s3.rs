use crate::utils::constants;
use rand::distributions::Alphanumeric;
use rand::Rng;
use tokio::io::AsyncReadExt as _;

#[derive(Debug, serde::Serialize, Clone)]
pub struct UploadedFile {
    pub(crate) filename: String,
    pub(crate) s3_key: String,
    pub s3_url: String,
}

impl UploadedFile {
    /// Construct new uploaded file info container.
    pub fn new(
        filename: impl Into<String>,
        s3_key: impl Into<String>,
        s3_url: impl Into<String>,
    ) -> Self {
        Self {
            filename: filename.into(),
            s3_key: s3_key.into(),
            s3_url: s3_url.into(),
        }
    }
}

/// S3 client wrapper to expose semantic upload operations.
#[derive(Debug, Clone)]
pub struct Client {
    s3: aws_sdk_s3::Client,
    bucket_name: String,
}

impl Client {
    pub fn new(config: aws_sdk_s3::Config) -> Client {
        Client {
            s3: aws_sdk_s3::Client::from_conf(config),
            bucket_name: (*constants::AWS_S3_BUCKET_NAME).clone(),
        }
    }

    pub fn url(&self, key: &str) -> String {
        format!(
            "https://{}.{}.digitaloceanspaces.com/{key}",
            (*constants::AWS_S3_BUCKET_NAME).clone(),
            (*constants::AWS_REGION).clone(),
        )
    }

    pub async fn upload(
        &self,
        file: &actix_multipart::form::tempfile::TempFile,
        key_prefix: &str,
    ) -> UploadedFile {
        let filename = file.file_name.as_deref().expect("TODO");
        let key = format!("{key_prefix}{filename}");
        let s3_url = self
            .put_object_from_file(file.file.path().to_str().unwrap(), &key)
            .await;
        UploadedFile::new(filename, key, s3_url)
    }

    async fn put_object_from_file(&self, local_path: &str, key: &str) -> String {
        let mut file = tokio::fs::File::open(local_path).await.unwrap();

        let size_estimate = file
            .metadata()
            .await
            .map(|md| md.len())
            .unwrap_or(1024)
            .try_into()
            .expect("file too big");

        let mut contents = Vec::with_capacity(size_estimate);
        file.read_to_end(&mut contents).await.unwrap();

        let _res = self
            .s3
            .put_object()
            .bucket(&self.bucket_name)
            .key(key)
            .body(aws_sdk_s3::primitives::ByteStream::from(contents))
            .acl("public-read".parse().unwrap())
            .send()
            .await
            .expect("Failed to put object");

        self.url(key)
    }

    pub async fn delete_file(&self, key: &str) -> bool {
        self.s3
            .delete_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await
            .is_ok()
    }
}

pub(crate) async fn configure_and_return_s3_client() -> Client {
    let aws_key = (*constants::AWS_ACCESS_KEY_ID).clone();
    let aws_key_secret = (*constants::AWS_SECRET_ACCESS_KEY).clone();
    let aws_cred = aws_sdk_s3::config::Credentials::new(
        aws_key,
        aws_key_secret,
        None,
        None,
        "loaded-from-custom-env",
    );

    let aws_region = aws_sdk_s3::config::Region::new((*constants::AWS_REGION).clone());
    let aws_config_builder = aws_sdk_s3::config::Builder::new()
        .region(aws_region)
        .credentials_provider(aws_cred)
        .endpoint_url(format!(
            "https://{}.digitaloceanspaces.com",
            (*constants::AWS_REGION).clone()
        ));

    let aws_config = aws_config_builder.build();
    Client::new(aws_config)
}

pub fn generate_random_session_id() -> String {
    let session_id: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();
    session_id
}
