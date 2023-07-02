use actix_web::HttpResponse;
use actix_web_flash_messages::FlashMessage;

use crate::routes::http_utils;
use crate::session::TypedSession;

pub async fn logout(
    session: TypedSession,
) -> Result<HttpResponse, actix_web::Error> {
    session.logout();
    FlashMessage::info("You have successfully logged out.").send();
    Ok(http_utils::seeother("/login"))
}
