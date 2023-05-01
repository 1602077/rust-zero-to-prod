use actix_web::HttpResponse;

#[tracing::instrument(name = "confirm a pending subscriber")]
pub async fn confirm() -> HttpResponse {
    HttpResponse::Ok().finish()
}
