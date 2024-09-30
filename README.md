# ConvoForge

## Overview
This project requires several environment variables to be set for proper configuration. These variables can be specified in a `.env` file or directly in the `docker-compose.yml` file. Below is a list of the required environment variables along with examples.

## Environment Variables

### 1. `DATABASE_URL`
- **Description**: The URL used to connect to the PostgreSQL database.
- **Example**: `postgres://postgres:postgres@postgres:5432/postgres`
- **Details**:
    - `postgres` is the username.
    - `postgres` is the password.
    - `postgres` is the hostname (in this case, referring to the service name in Docker).
    - `5432` is the default port for PostgreSQL.
    - `postgres` is the database name.

### 2. `CLIENT_SECRET`
- **Description**: A secret key used by the client for secure operations such as encryption or token generation.
- **Example**: `your-client-secret`
- **Details**: Can be any secure random string.

### 3. `JWT_SECRET`
- **Description**: A secret key used to sign and verify JSON Web Tokens (JWTs).
- **Example**: `your-jwt-secret`
- **Details**: This should be a strong, random string to ensure the security of JWTs.

**Note: Only DigitalOcean Spaces is supported for file storage at the moment.**

### 4. `AWS_S3_BUCKET_NAME`
- **Description**: The name of the AWS S3 bucket where files will be stored.
- **Example**: `your-s3-bucket-name`

### 5. `AWS_REGION`
- **Description**: The AWS region where your S3 bucket is hosted.
- **Example**: `us-west-2`

### 6. `AWS_ACCESS_KEY_ID`
- **Description**: Your AWS Access Key ID, used to authenticate your requests to AWS services.
- **Example**: `your-aws-access-key-id`

### 7. `AWS_SECRET_ACCESS_KEY`
- **Description**: Your AWS Secret Access Key, used along with the Access Key ID to authenticate your requests to AWS services.
- **Example**: `your-aws-secret-access-key`

## Setting Up Your Environment
Add the environment variables as follows (in docker-compose.yml or .env file):

```env
DATABASE_URL=
CLIENT_SECRET=
JWT_SECRET=
AWS_S3_BUCKET_NAME=
AWS_REGION=
AWS_ACCESS_KEY_ID=
AWS_SECRET_ACCESS_KEY=
