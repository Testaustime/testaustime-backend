use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use actix_web::test::{self, TestRequest};
use serde_json::json;

use super::{macros::*, *};
use crate::models::{NewUserIdentity, SecuredAccessTokenResponse, SelfUser};

#[actix_web::test]
async fn register_and_delete() {
    let app = test::init_service(App::new().configure(init_test_services)).await;
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 80u16);
    let body = json!({"username": "testuser", "password": "password"});

    let resp = request!(app, addr, post, "/auth/register", body);
    assert!(resp.status().is_success(), "Failed to create user");
    let user: NewUserIdentity = test::read_body_json(resp).await;

    let resp = request!(app, addr, post, "/auth/register", body);
    assert!(resp.status().is_client_error(), "Usernames must be unique");

    let resp = request_auth!(app, addr, get, "/users/@me", user.auth_token);
    assert!(
        resp.status().is_success(),
        "Authentication token should work"
    );

    let resp = request!(app, addr, delete, "/users/@me/delete", body);
    assert!(resp.status().is_success(), "Failed to delete user");

    let resp = request!(app, addr, post, "/auth/login", body);
    assert!(resp.status().is_client_error(), "User should be deleted")
}

#[actix_web::test]
async fn login_change_username_and_password() {
    let app = test::init_service(App::new().configure(init_test_services)).await;

    let body = json!({"username": "testuser2", "password": "password"});
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 80u16);

    let resp = request!(app, addr, post, "/auth/register", body);
    assert!(resp.status().is_success(), "Failed to create user");
    let user: NewUserIdentity = test::read_body_json(resp).await;

    let resp = request!(app, addr, post, "/auth/login", body);
    assert!(resp.status().is_success(), "Login failed");
    let login_user: SelfUser = test::read_body_json(resp).await;

    assert_eq!(
        user.auth_token, login_user.auth_token,
        "Auth tokens should be equal"
    );

    let resp = request!(app, addr, post, "/auth/securedaccess", body);
    assert!(
        resp.status().is_success(),
        "Getting secured access token failed"
    );

    let sat: SecuredAccessTokenResponse = test::read_body_json(resp).await;

    let change_request = json!({
        "new": "testuser3"
    });

    let resp = request_auth!(
        app,
        addr,
        post,
        "/auth/changeusername",
        user.auth_token,
        change_request
    );
    assert!(
        resp.status().is_client_error(),
        "Auth token is not secured access token"
    );

    let resp = request_auth!(
        app,
        addr,
        post,
        "/auth/changeusername",
        sat.token,
        change_request
    );
    assert!(resp.status().is_success(), "Username change failed");

    let resp = request_auth!(app, addr, get, "/users/@me", user.auth_token);
    let renamed_user: serde_json::Value = test::read_body_json(resp).await;
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
        addr,
        post,
        "/auth/changepassword",
        user.auth_token,
        pw_request
    );
    assert!(resp.status().is_success(), "Password change failed");

    let new_body = json!({"username": "testuser3", "password": "password1"});

    let resp = request!(app, addr, post, "/auth/login", new_body);
    assert!(resp.status().is_success(), "Password not changed");

    let resp = request!(app, addr, delete, "/users/@me/delete", new_body);
    assert!(resp.status().is_success(), "Failed to delete user");
}

#[actix_web::test]
async fn invalid_usernames_and_passwords_are_rejected() {
    let app = test::init_service(App::new().configure(init_test_services)).await;

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 80u16);

    let body = json!({"username": "invalid[$$]", "password": "password"});
    let resp = request!(app, addr, post, "/auth/register", body);
    assert!(
        resp.status().is_client_error(),
        "Invalid username should fail"
    );
    let resp_body: serde_json::Value = test::read_body_json(resp).await;

    assert!(resp_body["error"]
        .as_str()
        .unwrap()
        .to_ascii_lowercase()
        .contains("username"));

    let body = json!({"username": "validusername", "password": "short"});
    let resp = request!(app, addr, post, "/auth/register", body);
    assert!(
        resp.status().is_client_error(),
        "Too short password should fail"
    );
    let resp_body: serde_json::Value = test::read_body_json(resp).await;

    assert!(resp_body["error"]
        .as_str()
        .unwrap()
        .to_ascii_lowercase()
        .contains("password"));
}

// TODO: test ratelimits
