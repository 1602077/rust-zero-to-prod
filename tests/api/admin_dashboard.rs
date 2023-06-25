use crate::helpers::spawn_app;
use crate::login::assert_is_redirect_to;

#[tokio::test]
async fn you_must_be_logged_in_to_access_admin_dashboard() {
    let app = spawn_app().await;

    let response = app.get_admin_dashboard().await;

    assert_is_redirect_to(&response, "/login")
}

#[tokio::test]
async fn logout_clears_session_state() {
    let app = spawn_app().await;

    // act: login
    let login_body = serde_json::json!({
        "username": &app.test_user.username,
        "password": &app.test_user.password,
    });
    let resp = app.post_login(&login_body).await;
    assert_is_redirect_to(&resp, "/admin/dashboard");

    // act: follow the redirect
    let html_page = app.get_admin_dashboard_html().await;
    assert!(html_page.contains(&format!("Welcome {}", app.test_user.username)));

    // act: logout
    let resp = app.post_logout().await;
    assert_is_redirect_to(&resp, "/login");
    assert!(html_page
        .contains(r#"<p><i>You have successfully logged out.</i></p>"#));

    // act: attempt to load admin panel
    let resp = app.get_admin_dashboard().await;
    assert_is_redirect_to(&resp, "/login");
}
