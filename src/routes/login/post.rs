use actix_web::cookie::Cookie;
use actix_web::error::InternalError;
use actix_web::{web, HttpResponse, ResponseError};
use reqwest::header::LOCATION;
use reqwest::StatusCode;
use secrecy::Secret;
use sqlx::PgPool;

use crate::authentication::{validate_credentials, AuthError, Credentials};
use crate::routes::error_chain_fmt;

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: Secret<String>,
}

pub async fn login(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, InternalError<LoginError>> {
    let creds = Credentials {
        username: form.0.username,
        password: form.0.password,
    };

    tracing::Span::current()
        .record("username", &tracing::field::display(&creds.username));

    match validate_credentials(creds, &pool).await {
        Ok(user_id) => {
            tracing::Span::current()
                .record("user_id", &tracing::field::display(&user_id));

            Ok(HttpResponse::SeeOther()
                .insert_header((LOCATION, "/"))
                .finish())
        }
        Err(e) => {
            let e = match e {
                AuthError::InvalidCredentials(_) => {
                    LoginError::AuthError(e.into())
                }
                AuthError::UnexpectedError(_) => {
                    LoginError::UnexpectedError(e.into())
                }
            };
            let response = HttpResponse::SeeOther()
                .insert_header((LOCATION, "/login"))
                .cookie(Cookie::new("_flash", e.to_string()))
                .finish();

            Err(InternalError::from_response(e, response))
        }
    }
}

#[derive(thiserror::Error)]
pub enum LoginError {
    #[error("Authentication failed")]
    AuthError(#[source] anyhow::Error),

    #[error("Something went wrong")]
    UnexpectedError(#[from] anyhow::Error),
}

impl std::fmt::Debug for LoginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl ResponseError for LoginError {
    fn error_response(&self) -> HttpResponse<actix_web::body::BoxBody> {
        let encoded_error = urlencoding::Encoded::new(self.to_string());
        HttpResponse::build(self.status_code())
            .insert_header((
                LOCATION,
                format!("/login?error={}", encoded_error),
            ))
            .finish()
    }
    fn status_code(&self) -> reqwest::StatusCode {
        StatusCode::SEE_OTHER
    }
}