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
