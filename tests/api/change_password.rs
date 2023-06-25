use uuid::Uuid;

use crate::helpers::spawn_app;
use crate::login::assert_is_redirect_to;

#[tokio::test]
async fn must_be_logged_in_to_see_change_pwd_form() {
    let app = spawn_app().await;

    let response = app.get_change_password().await;

    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn must_be_logged_in_to_see_change_pwd() {
    let app = spawn_app().await;

    let new_password = Uuid::new_v4().to_string();

    let response = app
        .post_change_password(&serde_json::json!({
            "current_password": Uuid::new_v4().to_string(),
            "new_password": &new_password,
            "new_password_validate": &new_password,
        }))
        .await;

    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn new_password_fields_must_match() {
    let app = spawn_app().await;

    let new_password = Uuid::new_v4().to_string();
    let new_password_another_one = Uuid::new_v4().to_string();

    // act: login
    app.post_login(&serde_json::json! ({
        "username": &app.test_user.username,
        "password":&app.test_user.password,
    }))
    .await;

    // act: try to change password
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password":&app.test_user.password,
            "new_password":new_password,
            "new_password_validate":new_password_another_one,
        }))
        .await;

    assert_is_redirect_to(&resp, "/admin/password");

    // act: follow the redirect
    let html_page = app.post_change_password_html().await;
    assert!(html_page.contains("<p><i>Password fields must match.</i></p>"));
}
