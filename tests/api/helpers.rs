use once_cell::sync::Lazy;
use reqwest::Client;
use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;
use wiremock::MockServer;
use z2p::{
    configuration::{get_config, DatabaseSettings},
    startup::{get_connection_pool, Application},
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

// spawn_app launches application in the background.
pub async fn spawn_app() -> TestApp {
    // the first time initialise is called the code in tracing is invoked otherwise we skip.
    Lazy::force(&TRACING);

    let email_server = MockServer::start().await;

    let config = {
        let mut c = get_config().expect("failed to read configuration");
        c.database.database_name = Uuid::new_v4().to_string();
        c.application.port = 0; // use a random available port.
        c.email.base_url = email_server.uri();
        c
    };

    configure_db(&config.database).await;

    let application = Application::build(config.clone())
        .await
        .expect("failed to build application");
    let application_port = application.port();

    let _ = tokio::spawn(application.run_until_stopped());

    // api_client allows for taking advantage of connection pooling.
    let api_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        // .pool_idle_timeout(std::time::Duration::from_millis(500))
        .build()
        .unwrap();

    TestApp {
        address: format!("http://localhost:{}", application_port),
        port: application_port,
        pool: get_connection_pool(&config.database),
        email_server,
        api_client,
    }
}

async fn configure_db(config: &DatabaseSettings) -> PgPool {
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

pub struct TestApp {
    pub address: String,
    pub port: u16,
    pub pool: PgPool,
    pub email_server: MockServer,
    pub api_client: reqwest::Client,
}

/// Confirmation links embedded in the request to the email API.
pub struct ConfirmationLinks {
    pub html: reqwest::Url,
    pub plain_text: reqwest::Url,
}

impl TestApp {
    pub async fn health(&self, client: Client) -> reqwest::Response {
        dbg!(&self.address);
        client
            .get(&format!("{}/health", &self.address))
            .send()
            .await
            .expect("failed to execute request.")
    }

    /// Extract the confirmation links embedded in the request to the email API.
    pub fn get_confirmation_links(
        &self,
        email_request: &wiremock::Request,
    ) -> ConfirmationLinks {
        let body: serde_json::Value =
            serde_json::from_slice(&email_request.body).unwrap();

        // Extract the link from one of the request fields.
        let get_link = |s: &str| {
            let links: Vec<_> = linkify::LinkFinder::new()
                .links(s)
                .filter(|l| *l.kind() == linkify::LinkKind::Url)
                .collect();
            assert_eq!(links.len(), 1);
            let raw_link = links[0].as_str().to_owned();
            let mut confirmation_link = reqwest::Url::parse(&raw_link).unwrap();
            // Let's make sure we don't call random APIs on the web
            assert_eq!(confirmation_link.host_str().unwrap(), "127.0.0.1");
            confirmation_link.set_port(Some(self.port)).unwrap();
            confirmation_link
        };

        let html = get_link(body["HtmlBody"].as_str().unwrap());
        let plain_text = get_link(body["TextBody"].as_str().unwrap());
        ConfirmationLinks { html, plain_text }
    }

    pub async fn post_subscriptions(&self, body: String) -> reqwest::Response {
        self.api_client
            .post(&format!("{}/subscriptions", &self.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .expect("failed to execute request")
    }
}
