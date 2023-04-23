use std::net::TcpListener;

use sqlx::PgPool;
use z2p::{configuration::get_config, startup};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let config = get_config().expect("failed to read config");
    let addr = format!("127.0.0.1:{}", config.application_port);

    let listener = TcpListener::bind(addr).expect("failed to bind to port");
    let connection_pool = PgPool::connect(&config.database.connection_string())
        .await
        .expect("failed to connect to postgres");

    startup::run(listener, connection_pool)?.await
}
