use uuid::Uuid;
use zero2prod::routes::{MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH};

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
        "password": &app.test_user.password,
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

#[tokio::test]
async fn current_password_must_be_valid() {
    let app = spawn_app().await;

    let new_password = Uuid::new_v4().to_string();
    let wrong_password = Uuid::new_v4().to_string();

    // act: login
    app.post_login(&serde_json::json! ({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // act: try to change password
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": &wrong_password,
            "new_password": new_password,
            "new_password_validate": new_password,
        }))
        .await;

    assert_is_redirect_to(&resp, "/admin/password");

    // act: follow the redirect
    let html_page = app.post_change_password_html().await;
    assert!(html_page.contains("<p><i>Current password is incorrect.</i></p>"));
}

#[tokio::test]
async fn validate_new_password_is_in_correct_length_range() {
    let app = spawn_app().await;

    // too short a password.
    let new_password = "aaa".to_string();

    // act: login
    app.post_login(&serde_json::json! ({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    }))
    .await;

    // act: try to change password
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": new_password,
            "new_password_validate": new_password,
        }))
        .await;

    assert_is_redirect_to(&resp, "/admin/password");

    // act: follow the redirect
    let html_page = app.post_change_password_html().await;
    assert!(html_page.contains(&format!(
        "<p><i>New password must be between {} and {} characters.</i></p>",
        MIN_PASSWORD_LENGTH, MAX_PASSWORD_LENGTH,
    )))
}

#[tokio::test]
async fn changing_password_works() {
    let app = spawn_app().await;
    let new_password = Uuid::new_v4().to_string();

    // act: login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let resp = app.post_login(&login_body).await;
    assert_is_redirect_to(&resp, "/admin/dashboard");

    // act: change password
    let resp = app
        .post_change_password(&serde_json::json!({
            "current_password": &app.test_user.password,
            "new_password": &new_password,
            "new_password_validate": &new_password,
        }))
        .await;
    assert_is_redirect_to(&resp, "/admin/password");

    // act: follow the redirect
    let html_page = app.post_change_password_html().await;
    assert!(html_page.contains("<p><i>Your password has been changed.</i></p>"));

    // act: logout
    let resp = app.post_logout().await;
    assert_is_redirect_to(&resp, "/login");

    // act: follow the redirect
    let html_page = app.get_login_html().await;
    dbg!(&html_page);
    assert!(
        html_page.contains("<p><i>You have successfully logged out.</i></p>")
    );

    // act: login using the new password
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &new_password,
    });
    let resp = app.post_login(&login_body).await;
    assert_is_redirect_to(&resp, "/admin/dashboard");
}
