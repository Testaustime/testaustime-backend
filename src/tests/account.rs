use serde_json::json;

use super::{macros::*, *};
use crate::models::NewUserIdentity;

#[tokio::test]
async fn public_accounts() {
    let mut app = create_test_router().into_service();

    let body = json!({"username": "celebrity", "password": "password"});
    let resp = request!(app, POST, "/auth/register", body);
    assert!(resp.status().is_success(), "Creating user failed");

    let user: NewUserIdentity = body_to_json(resp).await;
    let resp = request_auth!(app, GET, "/users/@me", user.auth_token);

    assert!(resp.status().is_success(), "Getting profile failed");

    let profile: serde_json::Value = body_to_json(resp).await;
    assert!(
        !profile["is_public"].as_bool().unwrap(),
        "New account should be private"
    );

    let resp = request!(app, GET, "/users/celebrity/activity/data");
    assert!(
        resp.status().is_client_error(),
        "Data should be private for private accounts"
    );

    let change = json!({"public_profile": true});
    let resp = request_auth!(app, POST, "/account/settings", user.auth_token, change);

    assert!(resp.status().is_success(), "Changing settings failed");

    let resp = request_auth!(app, GET, "/users/@me", user.auth_token);

    assert!(resp.status().is_success(), "Getting profile failed");

    let profile: serde_json::Value = body_to_json(resp).await;
    assert!(
        profile["is_public"].as_bool().unwrap(),
        "Setting account public failed"
    );

    let resp = request!(app, GET, "/users/celebrity/activity/data");
    assert!(
        resp.status().is_success(),
        "Data should be public for public accounts"
    );

    let resp = request!(app, DELETE, "/users/@me/delete", body);
    assert!(resp.status().is_success(), "Failed to delete user");
}

// TODO: add test for searching public accounts
