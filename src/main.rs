use std::net::TcpListener;

use sqlx::postgres::PgPoolOptions;
use z2p::{
    configuration::get_config, email_client::EmailClient, startup, telemetry,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let subscriber = telemetry::get_subscriber(
        "zero2prod".into(),
        "info".into(),
        std::io::stdout,
    );
    telemetry::init_subscriber(subscriber);

    let config = get_config().expect("failed to read config");
    let addr =
        format!("{}:{}", config.application.host, config.application.port);

    let listener = TcpListener::bind(addr).expect("failed to bind to port");
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(config.database.with_db());

    let sender_email =
        config.email.sender().expect("inavlid sender email address");
    let timeout = config.email.timeout();
    let email_client = EmailClient::new(
        config.email.base_url,
        sender_email,
        config.email.auth_token,
        timeout,
    );

    startup::run(listener, connection_pool, email_client)?.await
}
