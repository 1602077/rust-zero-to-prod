use actix_web::error::InternalError;
use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use anyhow::anyhow;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;
use uuid::Uuid;

use crate::authentication::{
    self, validate_credentials, AuthError, Credentials,
};
use crate::routes::admin::dashboard::get_username;
use crate::routes::{e500, http_utils, seeother};
use crate::session::TypedSession;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_validate: Secret<String>,
}

pub const MIN_PASSWORD_LENGTH: usize = 12;
pub const MAX_PASSWORD_LENGTH: usize = 128;

pub async fn change_password(
    form: web::Form<FormData>,
    session: TypedSession,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, actix_web::Error> {
    let user_id = reject_anonymous_users(session).await?;

    if form.new_password.expose_secret()
        != form.new_password_validate.expose_secret()
    {
        FlashMessage::error("Password fields must match.").send();
        return Ok(seeother("/admin/password"));
    }
    let password_len = form.new_password.expose_secret().len();
    if !(MIN_PASSWORD_LENGTH..=MAX_PASSWORD_LENGTH).contains(&password_len) {
        FlashMessage::error(format!(
            "New password must be between {} and {} characters.",
            MIN_PASSWORD_LENGTH, MAX_PASSWORD_LENGTH,
        ))
        .send();
        return Ok(seeother("/admin/password"));
    };

    let username = get_username(user_id, &pool).await.map_err(e500)?;
    if let Err(e) = validate_credentials(
        Credentials {
            username,
            password: form.0.current_password,
        },
        &pool,
    )
    .await
    {
        return match e {
            AuthError::InvalidCredentials(_) => {
                FlashMessage::error("Current password is incorrect.").send();
                Ok(seeother("/admin/password"))
            }
            AuthError::UnexpectedError(_) => Err(e500(e)),
        };
    }

    authentication::change_password(user_id, form.0.new_password, &pool)
        .await
        .map_err(e500)?;
    FlashMessage::info("Your password has been changed.").send();

    Ok(http_utils::seeother("/admin/password"))
}

pub async fn reject_anonymous_users(
    session: TypedSession,
) -> Result<Uuid, actix_web::Error> {
    match session.get_user_id().map_err(http_utils::e500)? {
        Some(user_id) => Ok(user_id),
        None => {
            let resp = http_utils::seeother("/login");
            let e = anyhow!("User is not logged in.");
            Err(InternalError::from_response(e, resp).into())
        }
    }
}
