use uuid::Uuid;
use wiremock::matchers::{any, method, path};
use wiremock::{Mock, ResponseTemplate};

use crate::helpers::{spawn_app, ConfirmationLinks, TestApp};

#[tokio::test]
async fn newsletters_are_not_delivered_to_unconfirmed_subscribers() {
    let app = spawn_app().await;
    create_unconfirmed_subscriber(&app).await;

    Mock::given(any())
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&app.email_server)
        .await;

    let newsletter_req_body = serde_json::json!({
        "title": "Newsletter Title",
        "content": {
            "text": "newsletter body as plain text",
            "html": "<p>as html</p>",
        }
    });

    let resp = app.post_newsletters(newsletter_req_body).await;

    assert_eq!(resp.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_are_delivered_to_confirmed_subscribers() {
    let app = spawn_app().await;
    create_confirmed_subscriber(&app).await;

    Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    let newsletter_req_body = serde_json::json!({
        "title": "Newsletter Title",
        "content": {
            "text": "newsletter body as plain text",
            "html": "<p>as html</p>",
        }
    });

    let resp = app.post_newsletters(newsletter_req_body).await;

    assert_eq!(resp.status().as_u16(), 200);
}

#[tokio::test]
async fn newsletters_returns_400_for_invalid_data() {
    let app = spawn_app().await;
    let test_cases = vec![
        (
            serde_json::json!({
                "content": {
                    "text": "newsletters plain text",
                    "html": "<p>this ones html</p>",
                }
            }),
            "missing title",
        ),
        (
            serde_json::json!({
                "title": "what a title",
            }),
            "missing content",
        ),
    ];

    for (invalid_body, error_message) in test_cases {
        let resp = app.post_newsletters(invalid_body).await;

        assert_eq!(
            400,
            resp.status().as_u16(),
            "api did not fail with 400 bad request for a payload of {}",
            error_message
        )
    }
}

async fn create_unconfirmed_subscriber(app: &TestApp) -> ConfirmationLinks {
    let body = "name=j&email=j%40mail.com";

    let _mock_guard = Mock::given(path("/email"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .named("Create unconfirmed subscriber")
        .expect(1)
        .mount_as_scoped(&app.email_server)
        .await;

    app.post_subscriptions(body.into())
        .await
        .error_for_status()
        .unwrap();

    // inspect requests received by mock and pull out confirmation link.
    let email_request = &app
        .email_server
        .received_requests()
        .await
        .unwrap()
        .pop()
        .unwrap();

    app.get_confirmation_links(&email_request)
}
async fn create_confirmed_subscriber(app: &TestApp) {
    let confirmation_link = create_unconfirmed_subscriber(app).await;
    reqwest::get(confirmation_link.html)
        .await
        .unwrap()
        .error_for_status()
        .unwrap();
}

#[tokio::test]
async fn requests_missing_authorization_are_rejected() {
    let app = spawn_app().await;

    let response = reqwest::Client::new()
        .post(&format!("{}/newsletter", &app.address))
        .json(&serde_json::json!({
            "title": "Newsletter title",
            "content": {
                "text": "Newsletter body as plain text",
                "html": "<p>Newsletter body as HTML</p>",
            }
        }))
        .send()
        .await
        .expect("Failed to execute request.");

    assert_eq!(401, response.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        response.headers()["WWW-Authenticate"]
    );
}

#[tokio::test]
async fn non_existing_user_is_rejected() {
    let app = spawn_app().await;

    let user = Uuid::new_v4().to_string();
    let pass = Uuid::new_v4().to_string();

    let resp = reqwest::Client::new()
        .post(&format!("{}/newsletter", &app.address))
        .basic_auth(user, Some(pass))
        .json(&serde_json::json!({
            "title":"newsletter title",
            "content":{
                "text":"plain text body",
                "html":"<p>body</p>"
            }
        }))
        .send()
        .await
        .expect("failed to exeute request.");

    assert_eq!(401, resp.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        resp.headers()["WWW-Authenticate"]
    )
}

#[tokio::test]
async fn invalid_password_is_rejected() {
    let app = spawn_app().await;

    let user = &app.test_user.username;
    let pass = Uuid::new_v4().to_string();

    let resp = reqwest::Client::new()
        .post(&format!("{}/newsletter", &app.address))
        .basic_auth(user, Some(pass))
        .json(&serde_json::json!({
            "title":"newsletter title",
            "content":{
                "text":"plain text body",
                "html":"<p>body</p>"
            }
        }))
        .send()
        .await
        .expect("failed to exeute request.");

    assert_eq!(401, resp.status().as_u16());
    assert_eq!(
        r#"Basic realm="publish""#,
        resp.headers()["WWW-Authenticate"]
    )
}
