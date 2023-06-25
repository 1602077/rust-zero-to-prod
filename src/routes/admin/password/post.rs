use actix_web::{web, HttpResponse};
use actix_web_flash_messages::FlashMessage;
use secrecy::{ExposeSecret, Secret};

use crate::routes::seeother;

#[derive(serde::Deserialize)]
pub struct FormData {
    current_password: Secret<String>,
    new_password: Secret<String>,
    new_password_validate: Secret<String>,
}

pub async fn change_password(
    form: web::Form<FormData>,
) -> Result<HttpResponse, actix_web::Error> {
    if form.new_password.expose_secret()
        != form.new_password_validate.expose_secret()
    {
        FlashMessage::error("Password fields must match.").send();
        return Ok(seeother("/admin/password"));
    }
    todo!()
}
