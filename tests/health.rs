use std::net::TcpListener;

use once_cell::sync::Lazy;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use z2p::{
    configuration::{get_config, DatabaseSettings, Settings},
    email_client::EmailClient,
    startup::run,
    telemetry::{get_subscriber, init_subscriber},
};

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();

    // Use of a sink allow for logs to be dumped by default when running tests.
    // If you do need them use:
    // # `TEST_LOG=1 cargo test health_check_works | bunyan`
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::stdout,
        );
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::sink,
        );
        init_subscriber(subscriber);
    }
});

pub struct TestApp {
    pub address: String,
    pub pool: PgPool,
}

// spawn_app launches application in the background.
async fn spawn_app(config: &Settings) -> TestApp {
    // the first time initialise is called the code in tracing is invoked otherwise we skip.
    Lazy::force(&TRACING);

    let listener = TcpListener::bind("127.0.0.1:0")
        .expect("failed to bind to random port");
    let port = listener.local_addr().unwrap().port();
    let addr = format!("http://127.0.0.1:{}", port);

    let conn_pool = configure_db(&config.database).await;

    let sender_email =
        config.email.sender().expect("inavlid sender email address");
    let timeout = config.email.timeout();
    let email_client = EmailClient::new(
        config.email.base_url.clone(),
        sender_email,
        config.email.auth_token.to_owned(),
        timeout,
    );

    let server = run(listener, conn_pool.clone(), email_client)
        .expect("failed to bind address");

    let _ = tokio::spawn(server);

    TestApp {
        address: addr,
        pool: conn_pool,
    }
}

pub async fn configure_db(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect_with(&config.without_db())
        .await
        .expect("failed to connect to postgres");

    connection
        .execute(
            format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str(),
        )
        .await
        .expect("failed to create database");
    println!("pool config {:#?}", config.with_db());
    let connection_pool = PgPool::connect_with(config.with_db())
        .await
        .expect("failed to create postgres connection pool");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("failed to migrate database");

    connection_pool
}

#[tokio::test]
async fn health_check_works() {
    let mut config = get_config().expect("failed to read configuration file");
    config.database.database_name = Uuid::new_v4().to_string();

    let app = spawn_app(&config).await;
    let client = reqwest::Client::new();

    let resp = client
        .get(&format!("{}/health", &app.address))
        .send()
        .await
        .expect("failed to execute request.");

    assert!(resp.status().is_success());
    assert_eq!(Some(0), resp.content_length());
}

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let mut config = get_config().expect("failed to read config file");
    config.database.database_name = Uuid::new_v4().to_string();

    let app = spawn_app(&config).await;

    let client = reqwest::Client::new();

    let body = "name=le%20guin&email=urlsula_le_guin%40gmail.com";

    let resp = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(200, resp.status().as_u16());

    let saved = sqlx::query!(r#"SELECT email, name FROM subscriptions"#)
        .fetch_one(&app.pool)
        .await
        .expect("failed to fetch saved subscription");

    assert_eq!(saved.email, "urlsula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_present_but_empty() {
    let mut config = get_config().expect("failed to read configuration file");
    config.database.database_name = Uuid::new_v4().to_string();

    let app = spawn_app(&config).await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=&email=email%40@mail.com", "empty name"),
        ("name=&helloemail=", "empty email"),
        ("name=&helloemail=not-an-email", "invalid email"),
    ];

    for (body, desc) in test_cases {
        let resp = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("failed to execute request.");

        assert_eq!(
            400,
            resp.status().as_u16(),
            "api did not return a 200 for payload {}",
            desc
        )
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_data_is_missing() {
    let mut config = get_config().expect("failed to read config file");
    config.database.database_name = Uuid::new_v4().to_string();

    let app = spawn_app(&config).await;
    let client = reqwest::Client::new();

    let test_cases = vec![
        ("name=jack%20m", "missing the email"),
        ("email=jcm%40gmail.com", "missing the name"),
        ("", "missing both name and email"),
    ];

    for (invalid_body, err_message) in test_cases {
        let resp = client
            .post(&format!("{}/subscriptions", &app.address))
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
