use serde_json::json;

use super::{macros::*, *};
use crate::models::{NewUserIdentity, SelfUser};

#[tokio::test]
async fn register_and_delete() {
    let mut app = create_test_router().into_service();

    let body = json!({"username": "testuser", "password": "password"});

    let resp = request!(app, POST, "/auth/register", body);
    assert!(resp.status().is_success(), "Failed to create user");
    let user: NewUserIdentity = body_to_json(resp).await;

    let resp = request!(app, POST, "/auth/register", body);
    assert!(resp.status().is_client_error(), "Usernames must be unique");

    let resp = request_auth!(app, GET, "/users/@me", user.auth_token);
    assert!(
        resp.status().is_success(),
        "Authentication token should work"
    );

    let resp = request!(app, DELETE, "/users/@me/delete", body);
    assert!(resp.status().is_success(), "Failed to delete user");

    let resp = request!(app, POST, "/auth/login", body);
    assert!(resp.status().is_client_error(), "User should be deleted")
}

#[tokio::test]
async fn login_change_username_and_password() {
    let mut app = create_test_router().into_service();

    let body = json!({"username": "testuser2", "password": "password"});

    let resp = request!(app, POST, "/auth/register", body);
    assert!(resp.status().is_success(), "Failed to create user");
    let user: NewUserIdentity = body_to_json(resp).await;

    let resp = request!(app, POST, "/auth/login", body);
    assert!(resp.status().is_success(), "Login failed");
    let login_user: SelfUser = body_to_json(resp).await;

    assert_eq!(
        user.auth_token, login_user.auth_token,
        "Auth tokens should be equal"
    );

    let change_request = json!({
        "new": "testuser3"
    });

    let resp = request_auth!(
        app,
        POST,
        "/auth/change-username",
        user.auth_token,
        change_request
    );
    assert!(resp.status().is_success(), "Username change failed");

    let resp = request_auth!(app, GET, "/users/@me", user.auth_token);
    let renamed_user: serde_json::Value = body_to_json(resp).await;
    assert_eq!(
        renamed_user["username"], change_request["new"],
        "Username not changed"
    );

    let pw_request = json!({
        "old": "password",
        "new": "password1",
    });

    let resp = request_auth!(
        app,
        POST,
        "/auth/change-password",
        user.auth_token,
        pw_request
    );
    assert!(resp.status().is_success(), "Password change failed");

    let new_body = json!({"username": "testuser3", "password": "password1"});

    let resp = request!(app, POST, "/auth/login", new_body);
    assert!(resp.status().is_success(), "Password not changed");

    let resp = request!(app, DELETE, "/users/@me/delete", new_body);
    assert!(resp.status().is_success(), "Failed to delete user");
}

#[tokio::test]
async fn invalid_usernames_and_passwords_are_rejected() {
    let mut app = create_test_router().into_service();

    let body = json!({"username": "invalid[$$]", "password": "password"});
    let resp = request!(app, POST, "/auth/register", body);

    assert!(
        resp.status().is_client_error(),
        "Invalid username should fail"
    );
    let resp_body: serde_json::Value = body_to_json(resp).await;

    assert!(resp_body["error"]
        .as_str()
        .unwrap()
        .to_ascii_lowercase()
        .contains("username"));

    let body = json!({"username": "validusername", "password": "short"});
    let resp = request!(app, POST, "/auth/register", body);
    assert!(
        resp.status().is_client_error(),
        "Too short password should fail"
    );
    let resp_body: serde_json::Value = body_to_json(resp).await;

    assert!(resp_body["error"]
        .as_str()
        .unwrap()
        .to_ascii_lowercase()
        .contains("password"));
}

// TODO: test ratelimits
