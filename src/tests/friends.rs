use serde_json::json;

use super::{macros::*, *};
use crate::models::NewUserIdentity;

#[tokio::test]
async fn adding_friends_works() {
    let mut app = create_test_router().into_service();

    let f1_body = json!({"username": "friend1", "password": "password"});
    let f2_body = json!({"username": "friend2", "password": "password"});

    let resp = request!(app, POST, "/auth/register", f1_body);
    assert!(resp.status().is_success(), "Failed to create user");
    let f1: NewUserIdentity = body_to_json(resp).await;

    let resp = request!(app, POST, "/auth/register", f2_body);
    assert!(resp.status().is_success(), "Failed to create user");
    let f2: NewUserIdentity = body_to_json(resp).await;

    let friend_body = json!({"code": f2.friend_code.clone()});
    let resp = request_auth!(app, POST, "/friends/add", f1.auth_token, friend_body);
    println!("{resp:?}");

    assert!(resp.status().is_success(), "Adding friend works");

    let resp = request_auth!(app, POST, "/friends/add", f1.auth_token, friend_body);

    assert!(resp.status().is_client_error(), "Re-adding friend fails");

    let self_body = json!({"code": f1.friend_code.clone()});
    let resp = request_auth!(app, POST, "/friends/add", f1.auth_token, self_body);

    assert!(resp.status().is_client_error(), "Adding self fails");

    let resp = request_auth!(
        app,
        GET,
        &format!("/users/{}/activity/data", &f1.username),
        f2.auth_token
    );
    assert!(
        resp.status().is_success(),
        "Friends can see eachothers data"
    );

    let resp = request_auth!(app, GET, "/friends/list", f2.auth_token);
    assert!(resp.status().is_success(), "Getting friends-list works");

    let friends: Vec<serde_json::Value> = body_to_json(resp).await;
    assert_eq!(friends.len(), 1, "Friend appears in friends-list");

    let resp = request!(app, DELETE, "/users/@me/delete", f1_body);
    assert!(resp.status().is_success(), "Failed to delete user");

    let resp = request!(app, DELETE, "/users/@me/delete", f2_body);
    assert!(resp.status().is_success(), "Failed to delete user");
}

// TODO: write tests for /friends/regenerate and /friends/remove
