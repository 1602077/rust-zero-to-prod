use std::net::TcpListener;

use z2p::startup;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8000").expect("failed to bind to port");
    startup::run(listener)?.await
}
