use actix_web::http::header;
use actix_web::{web, HttpRequest, HttpResponse, ResponseError};
use anyhow::Context;
use reqwest::header::HeaderValue;
use reqwest::StatusCode;
use sqlx::PgPool;

use crate::authentication::{
    basic_authentication, validate_credentials, AuthError,
};
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

    let user_id =
        validate_credentials(creds, &pool)
            .await
            .map_err(|e| match e {
                AuthError::InvalidCredentials(_) => {
                    PublishError::AuthError(e.into())
                }
                AuthError::UnexpectedError(_) => {
                    PublishError::UnexpectedError(e.into())
                }
            })?;

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
