use std::net::TcpListener;

use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;
use z2p::{configuration::get_config, startup, telemetry};

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
        .connect_lazy(&config.database.connection_string().expose_secret())
        .expect("failed to connect to postgres connection pool");

    startup::run(listener, connection_pool)?.await
}
