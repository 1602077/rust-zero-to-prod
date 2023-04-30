use uuid::Uuid;
use z2p::configuration::get_config;

use crate::helpers::spawn_app;

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
