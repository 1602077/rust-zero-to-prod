use actix_web::{web, HttpResponse};
use sqlx::PgPool;

#[derive(serde::Deserialize)]
pub struct FormData {
    email: String,
    name: String,
}

#[tracing::instrument(
    name = "adding a new subscriber",
    skip(form,pool),
    fields(
        name = %form.name,
        email=%form.email,
    )
)]
pub async fn subscribe(
    form: web::Form<FormData>,
    pool: web::Data<PgPool>,
) -> HttpResponse {
    match insert_subscriber(&form, &pool).await {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

#[tracing::instrument(
    name = "saving a new subscriber details to database",
    skip(form, pool)
)]
pub async fn insert_subscriber(
    form: &FormData,
    pool: &PgPool,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO subscriptions (id,email,name,subscribed_at)
        VALUES ($1, $2, $3, $4)
        "#,
        uuid::Uuid::new_v4(),
        form.email,
        form.name,
        chrono::Utc::now()
    )
    .execute(pool)
    .await
    .map_err(|e| {
        tracing::error!("failed to execute query: {:?}", e);
        e
    })?;

    Ok(())
}
