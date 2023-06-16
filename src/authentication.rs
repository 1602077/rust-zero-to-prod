use actix_web::http::header::HeaderMap;
use anyhow::{anyhow, Context};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::Engine;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::telemetry::spawn_blocking_with_tracing;

pub struct Credentials {
    pub username: String,
    pub password: Secret<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("Invalid credentials.")]
    InvalidCredentials(#[source] anyhow::Error),

    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}

#[tracing::instrument(name = "validate credentials", skip(credentials, pool))]
pub async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, AuthError> {
    let mut user_id = None;

    // prevent timing attack with a default hash to be calculated.
    let mut expected_password_hash = Secret::new(
        "$argon2id$v=19$m=15000,t=2,p=1$\
        gZiV/M1gPc22ElAH/Jh1Hw$\
        CWOrkoo7oJBQ/iyh7uJ0LO2aLEfrHwTwllSAxT0zRno"
            .to_string(),
    );

    if let Some((stored_user_id, stored_pwd_hash)) =
        get_stored_credentials(&credentials.username, pool)
            .await
            .map_err(AuthError::UnexpectedError)?
    {
        user_id = Some(stored_user_id);
        expected_password_hash = stored_pwd_hash;
    };

    // calculating hashes is CPU intensive (order of 10ms to hash), therefore
    // launch in a separate thread pool.
    spawn_blocking_with_tracing(move || {
        verify_password_hash(expected_password_hash, credentials.password)
    })
    .await
    .context("failed to spawn blocking task.")??;

    user_id
        .ok_or_else(|| anyhow!("unknown username."))
        .map_err(AuthError::InvalidCredentials)
}
#[tracing::instrument(
    name = "verify password hash",
    skip(expected_password_hash, password_candidate)
)]
pub fn verify_password_hash(
    expected_password_hash: Secret<String>,
    password_candidate: Secret<String>,
) -> Result<(), AuthError> {
    let expected_password_hash =
        PasswordHash::new(expected_password_hash.expose_secret())
            .context("failed to parse hash to PHC string format.")?;

    Argon2::default()
        .verify_password(
            password_candidate.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("invalid password")
        .map_err(AuthError::InvalidCredentials)
}

#[tracing::instrument(name = "get stored credentials", skip(username, pool))]
pub async fn get_stored_credentials(
    username: &str,
    pool: &PgPool,
) -> Result<Option<(uuid::Uuid, Secret<String>)>, anyhow::Error> {
    let row: Option<_> = sqlx::query!(
        r#"
        SELECT user_id, password_hash
        FROM users
        WHERE username = $1 
        "#,
        username
    )
    .fetch_optional(pool)
    .await
    .context("failed to perform query to retrieve stored credentials")?
    .map(|row| (row.user_id, Secret::new(row.password_hash)));

    Ok(row)
}

pub fn basic_authentication(
    headers: &HeaderMap,
) -> Result<Credentials, anyhow::Error> {
    let header_value = headers
        .get("Authorization")
        .context("The 'Authorization' header was missing")?
        .to_str()
        .context("The 'Authorization' header was not a valid UTF8 string.")?;

    let base64encoded_credentials = header_value
        .strip_prefix("Basic ")
        .context("The authorization scheme was not 'Basic'.")?;

    let decoded_credentials = base64::engine::general_purpose::STANDARD
        .decode(base64encoded_credentials)
        .context("Failed to base64-decode 'Basic' credentials.")?;

    let decoded_credentials = String::from_utf8(decoded_credentials)
        .context("The decoded credential string is valid UTF8.")?;

    let mut credentials = decoded_credentials.splitn(2, ':');
    let username = credentials
        .next()
        .ok_or_else(|| {
            anyhow::anyhow!("A username must be provided in 'Basic' auth.")
        })?
        .to_string();
    let password = credentials
        .next()
        .ok_or_else(|| {
            anyhow::anyhow!("A password must be provided in 'Basic' auth.")
        })?
        .to_string();

    Ok(Credentials {
        username,
        password: Secret::new(password),
    })
}
