use std::time::Duration;

use serde_json::json;

use super::{macros::*, *};
use crate::models::{CurrentActivity, HeartBeat, NewUserIdentity};

#[tokio::test]
async fn updating_activity_works() {
    let mut app = create_test_router().into_service();

    let body = json!({"username": "activeuser", "password": "password"});
    let resp = request!(app, POST, "/auth/register", body);
    let user: NewUserIdentity = body_to_json(resp).await;

    let heartbeat = HeartBeat {
        hostname: Some(String::from("hostname")),
        project_name: Some(String::from("cool project")),
        language: Some(String::from("rust")),
        editor_name: Some(String::from("nvim")),
        hidden: Some(false),
    };

    let resp = request_auth!(app, POST, "/activity/update", user.auth_token, heartbeat);
    assert!(
        resp.status().is_success(),
        "Sending heartbeat should succeed"
    );

    let resp = request_auth!(app, GET, "/users/@me/activity/current", user.auth_token);
    assert!(
        resp.status().is_success(),
        "Getting current activity should work"
    );

    let current: CurrentActivity = body_to_json(resp).await;

    assert_eq!(
        heartbeat, current.heartbeat,
        "Active session should match the sent heartbeat"
    );

    // NOTE: adding duration to the session
    tokio::time::sleep(Duration::from_secs(1)).await;

    let resp = request_auth!(app, POST, "/activity/update", user.auth_token, heartbeat);
    assert!(resp.status().is_success(), "Extending session should work");

    let resp = request_auth!(app, GET, "/users/@me/activity/current", user.auth_token);
    let current: CurrentActivity = body_to_json(resp).await;
    assert!(
        current.duration >= 1,
        "Duration should be at least 1 second"
    );

    let new_heartbeat = HeartBeat {
        hostname: Some(String::from("hostname")),
        project_name: Some(String::from("another project")),
        language: Some(String::from("rust")),
        editor_name: Some(String::from("nvim")),
        hidden: Some(false),
    };
    let resp = request_auth!(
        app,
        POST,
        "/activity/update",
        user.auth_token,
        new_heartbeat
    );
    assert!(
        resp.status().is_success(),
        "Sending heartbeat should succeed"
    );

    let resp = request_auth!(app, GET, "/users/@me/activity/current", user.auth_token);
    let current: CurrentActivity = body_to_json(resp).await;
    assert!(
        current.heartbeat == new_heartbeat,
        "Mismatch should start new session"
    );

    let resp = request_auth!(app, GET, "/users/@me/activity/data", user.auth_token);
    let data: Vec<serde_json::Value> = body_to_json(resp).await;

    assert!(!data.is_empty(), "Old session is stored in the database");

    let resp = request!(app, DELETE, "/users/@me/delete", body);
    assert!(resp.status().is_success(), "Failed to delete user");
}

#[tokio::test]
async fn flushing_works() {
    let mut app = create_test_router().into_service();

    let body = json!({"username": "activeuser2", "password": "password"});
    let resp = request!(app, POST, "/auth/register", body);
    let user: NewUserIdentity = body_to_json(resp).await;

    let heartbeat = HeartBeat {
        hostname: Some(String::from("hostname")),
        project_name: Some(String::from("cool project")),
        language: Some(String::from("rust")),
        editor_name: Some(String::from("nvim")),
        hidden: Some(false),
    };

    let resp = request_auth!(app, POST, "/activity/update", user.auth_token, heartbeat);
    assert!(
        resp.status().is_success(),
        "Sending heartbeat should succeed"
    );

    let resp = request_auth!(app, GET, "/users/@me/activity/data", user.auth_token);
    let data: Vec<serde_json::Value> = body_to_json(resp).await;

    assert!(data.is_empty(), "No session should exist");

    let resp = request_auth!(app, POST, "/activity/flush", user.auth_token);
    assert!(resp.status().is_success(), "Flushing should work");

    let resp = request_auth!(app, GET, "/users/@me/activity/data", user.auth_token);
    let data: Vec<serde_json::Value> = body_to_json(resp).await;

    assert!(!data.is_empty(), "Session should be saved after a flush");

    let resp = request!(app, DELETE, "/users/@me/delete", body);
    assert!(resp.status().is_success(), "Failed to delete user");
}

#[tokio::test]
async fn hidden_project() {
    let mut app = create_test_router().into_service();

    let body = json!({"username": "activeuser3", "password": "password"});
    let resp = request!(app, POST, "/auth/register", body);
    let user: NewUserIdentity = body_to_json(resp).await;

    let heartbeat = HeartBeat {
        hostname: Some(String::from("nsa-supercomputer")),
        project_name: Some(String::from("prism2")),
        language: Some(String::from("rust")),
        editor_name: Some(String::from("nvim")),
        hidden: Some(true),
    };

    let resp = request_auth!(app, POST, "/activity/update", user.auth_token, heartbeat);
    assert!(
        resp.status().is_success(),
        "Sending heartbeat should succeed"
    );

    let resp = request_auth!(app, GET, "/users/@me/activity/data", user.auth_token);
    let data: Vec<serde_json::Value> = body_to_json(resp).await;

    assert!(data.is_empty(), "No session should exist");

    let resp = request_auth!(app, POST, "/activity/flush", user.auth_token);
    assert!(resp.status().is_success(), "Flushing should work");

    let change = json!({"public_profile": true});
    let resp = request_auth!(app, POST, "/account/settings", user.auth_token, change);

    assert!(resp.status().is_success(), "Making profile public failed");

    let resp = request!(
        app,
        GET,
        "/users/activeuser3/activity/data",
        user.auth_token
    );
    let data: Vec<serde_json::Value> = body_to_json(resp).await;

    assert!(!data.is_empty(), "Session should be saved after a flush");
    // Print the actual value of the project name to see what it is
    assert!(
        data[0].get("project_name").unwrap_or(&json!("not_hidden")) == &json!("hidden"),
        "Activity project name should be empty string"
    );

    let resp = request!(app, DELETE, "/users/@me/delete", body);
    assert!(resp.status().is_success(), "Failed to delete user");
}

// TODO: write tests for /activity/delete and /activity/rename and /activity/hide
