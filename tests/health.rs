use std::net::TcpListener;

#[tokio::test]
async fn test_health() {
    let address = spawn_app();

    let client = reqwest::Client::new();

    let resp = client
        .get(&format!("{}/health", &address))
        .send()
        .await
        .expect("failed to execute request.");

    assert!(resp.status().is_success());
    assert_eq!(Some(0), resp.content_length());
}

// spawn_app launches application in the background.
fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let server = z2p::run(listener).expect("failed to bind address");

    let _ = tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}
