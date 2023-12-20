use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use actix_web::test::{self, TestRequest};
use serde_json::json;

use super::{macros::*, *};
use crate::models::{NewUserIdentity, SecuredAccessTokenResponse};

#[actix_web::test]
async fn public_accounts() {
    let app = test::init_service(App::new().configure(init_test_services)).await;

    let body = json!({"username": "celebrity", "password": "password"});
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 80u16);
    let resp = request!(app, addr, post, "/auth/register", body);
    assert!(resp.status().is_success(), "Creating user failed");

    let user: NewUserIdentity = test::read_body_json(resp).await;
    let resp = request_auth!(app, addr, get, "/users/@me", user.auth_token);

    assert!(resp.status().is_success(), "Getting profile failed");

    let profile: serde_json::Value = test::read_body_json(resp).await;
    assert!(
        !profile["is_public"].as_bool().unwrap(),
        "New account should be private"
    );

    let resp = request!(app, addr, get, "/users/celebrity/activity/data");
    assert!(
        resp.status().is_client_error(),
        "Data should be private for private accounts"
    );

    let resp = request!(app, addr, post, "/auth/securedaccess", body);
    assert!(
        resp.status().is_success(),
        "Getting secured access token failed"
    );
    let sat: SecuredAccessTokenResponse = test::read_body_json(resp).await;

    let change = json!({"public_profile": true});
    let resp = request_auth!(app, addr, post, "/account/settings", sat.token, change);

    assert!(resp.status().is_success(), "Changing settings failed");

    let resp = request_auth!(app, addr, get, "/users/@me", user.auth_token);

    assert!(resp.status().is_success(), "Getting profile failed");

    let profile: serde_json::Value = test::read_body_json(resp).await;
    assert!(
        profile["is_public"].as_bool().unwrap(),
        "Setting account public failed"
    );

    let resp = request!(app, addr, get, "/users/celebrity/activity/data");
    assert!(
        resp.status().is_success(),
        "Data should be public for public accounts"
    );

    let resp = request!(app, addr, delete, "/users/@me/delete", body);
    assert!(resp.status().is_success(), "Failed to delete user");
}

// TODO: add test for searching public accounts
