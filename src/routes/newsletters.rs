use actix_web::http::header::{self, HeaderMap};
use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use anyhow::{anyhow, Context};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use base64::Engine;
use reqwest::header::HeaderValue;
use reqwest::StatusCode;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::domain::SubscriberEmail;
use crate::email_client::EmailClient;
use crate::routes::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct BodyData {
    title: String,
    content: Content,
}

#[derive(serde::Deserialize)]
pub struct Content {
    html: String,
    text: String,
}

struct ConfirmedSubscriber {
    email: SubscriberEmail,
}

#[derive(thiserror::Error)]
pub enum PublishError {
    #[error("authentication failed")]
    AuthError(#[source] anyhow::Error),
    #[error(transparent)]
    UnexpectedError(#[from] anyhow::Error),
}
impl std::fmt::Debug for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for PublishError {
    fn error_response(&self) -> HttpResponse {
        match self {
            PublishError::UnexpectedError(_) => {
                HttpResponse::new(StatusCode::INTERNAL_SERVER_ERROR)
            }

            PublishError::AuthError(_) => {
                let mut resp = HttpResponse::new(StatusCode::UNAUTHORIZED);

                let header_value =
                    HeaderValue::from_str(r#"Basic realm="publish""#).unwrap();

                resp.headers_mut()
                    .insert(header::WWW_AUTHENTICATE, header_value);

                resp
            }
        }
    }
}

#[tracing::instrument(
    name = "publish a newsletter issue",
    skip(body, pool, email_client, request),
    fields(username=tracing::field::Empty, user_id=tracing::field::Empty)
)]
pub async fn publish_newsletter(
    body: web::Json<BodyData>,
    pool: web::Data<PgPool>,
    email_client: web::Data<EmailClient>,
    request: HttpRequest,
) -> Result<HttpResponse, PublishError> {
    let creds = basic_authentication(request.headers())
        .map_err(PublishError::AuthError)?;

    tracing::Span::current()
        .record("username", &tracing::field::display(&creds.username));

    let user_id = validate_credentials(creds, &pool).await?;

    tracing::Span::current()
        .record("user_id", &tracing::field::display(&user_id));

    let subscribers = get_confirmed_subscribers(&pool).await?;

    for subscriber in subscribers {
        match subscriber {
            Ok(subscriber) => email_client
                .send_email(
                    &subscriber.email,
                    &body.title,
                    &body.content.html,
                    &body.content.text,
                )
                .await
                .with_context(|| {
                    format!(
                        "failed to send newsltter issue to {}",
                        subscriber.email
                    )
                })?,
            Err(error) => {
                tracing::warn!(
                    error.cauase_chain=?error,
                    "Skipping a confirmed subscriber. \
                    Invalid contact details");
            }
        }
    }

    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(name = "get confirmed subscribers", skip(pool))]
async fn get_confirmed_subscribers(
    pool: &PgPool,
) -> Result<Vec<Result<ConfirmedSubscriber, anyhow::Error>>, anyhow::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT email 
        FROM subscriptions 
        WHERE status='confirmed';
        "#,
    )
    .fetch_all(pool)
    .await?;

    // map rows to domain type.
    let confirmed_subscribers = rows
        .into_iter()
        .map(|r| match SubscriberEmail::parse(r.email) {
            Ok(email) => Ok(ConfirmedSubscriber { email }),
            Err(error) => Err(anyhow::anyhow!(error)),
        })
        .collect();

    Ok(confirmed_subscribers)
}

pub struct Credentials {
    username: String,
    password: Secret<String>,
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

async fn validate_credentials(
    credentials: Credentials,
    pool: &PgPool,
) -> Result<uuid::Uuid, PublishError> {
    let row: Option<_> = sqlx::query!(
        r#"
        SELECT user_id, password_hash, salt
        FROM users
        WHERE username = $1 
        "#,
        credentials.username
    )
    .fetch_optional(pool)
    .await
    .context("failed to perform query to retrieve stored credentials")
    .map_err(PublishError::UnexpectedError)?;

    let (expected_password_hash, user_id) = match row {
        Some(row) => (row.password_hash, row.user_id),
        None => {
            return Err(PublishError::AuthError(anyhow!("unknown username")));
        }
    };

    let expected_password_hash = PasswordHash::new(&expected_password_hash)
        .context("failed to parse hash to PHC string format.")
        .map_err(PublishError::UnexpectedError)?;

    Argon2::default()
        .verify_password(
            credentials.password.expose_secret().as_bytes(),
            &expected_password_hash,
        )
        .context("invalid password")
        .map_err(PublishError::AuthError)?;

    Ok(user_id)
}
