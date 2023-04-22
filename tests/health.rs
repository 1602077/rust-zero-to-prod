use std::net::TcpListener;

use sqlx::{Connection, PgConnection};
use z2p::{configuration::get_config, startup::run};

// spawn_app launches application in the background.
fn spawn_app() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let server = run(listener).expect("failed to bind address");

    let _ = tokio::spawn(server);

    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_works() {
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

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    // setup
    let address = spawn_app();
    let config = get_config().expect("failed to read config file");
    let connection_addr = config.database.connection_string();

    let mut connection = PgConnection::connect(&connection_addr)
        .await
        .expect("failed to connect to postgres");

    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=urlsula_le_guin%40gmail.com";

    let resp = client
        .post(&format!("{}/subscriptions", &address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(200, resp.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&mut connection)
        .await
        .expect("failed to fetch saved subscription");

    assert_eq!(saved.email, "urlsula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let address = spawn_app();
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=jack%20m", "missing the email"),
        ("email=jcm%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, err_message) in test_cases {
        let resp = client
            .post(&format!("{}/subscriptions", &address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("failed to execute request");

        assert_eq!(
            400,
            resp.status().as_u16(),
            "The API did not fail with 400 Bad request when for a payload of {}",
            err_message
        )
    }
}
