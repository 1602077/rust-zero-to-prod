use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};
use sqlx::PgPool;

use crate::authentication::{
    self, validate_credentials, AuthError, Credentials, UserId,
};
use crate::routes::admin::dashboard::get_username;
use crate::routes::{e500, http_utils, seeother};

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
    pool: web::Data<PgPool>,
    user_id: web::ReqData<UserId>,
) -> Result<HttpResponse, actix_web::Error> {
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

    let username = get_username(**user_id, &pool).await.map_err(e500)?;
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

    authentication::change_password(**user_id, form.0.new_password, &pool)
        .await
        .map_err(e500)?;
    FlashMessage::info("Your password has been changed.").send();

    Ok(http_utils::seeother("/admin/password"))
}
