use uuid::Uuid;
use z2p::configuration::get_config;

use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_a_200_for_valid_form_data() {
    let mut config = get_config().expect("failed to read config file");
    config.database.database_name = Uuid::new_v4().to_string();

    let app = spawn_app().await;

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

    let app = spawn_app().await;
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

    let app = spawn_app().await;
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
