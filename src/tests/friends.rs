use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use actix_web::test::{self, TestRequest};
use serde_json::json;

use super::{macros::*, *};
use crate::models::NewUserIdentity;

#[actix_web::test]
async fn adding_friends_works() {
    let app = test::init_service(App::new().configure(init_test_services)).await;

    let f1_body = json!({"username": "friend1", "password": "password"});
    let f2_body = json!({"username": "friend2", "password": "password"});
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 80u16);
    let other_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 80u16);

    let resp = request!(app, addr, post, "/auth/register", f1_body);
    assert!(resp.status().is_success(), "Failed to create user");
    let f1: NewUserIdentity = test::read_body_json(resp).await;

    let resp = request!(app, other_addr, post, "/auth/register", f2_body);
    assert!(resp.status().is_success(), "Failed to create user");
    let f2: NewUserIdentity = test::read_body_json(resp).await;

    let resp = TestRequest::post()
        .peer_addr(addr)
        .uri("/friends/add")
        .insert_header(("authorization", "Bearer ".to_owned() + &f1.auth_token))
        .set_payload(f2.friend_code.clone())
        .send_request(&app)
        .await;

    assert!(resp.status().is_success(), "Adding friend works");

    let resp = TestRequest::post()
        .peer_addr(addr)
        .uri("/friends/add")
        .insert_header(("authorization", "Bearer ".to_owned() + &f1.auth_token))
        .set_payload(f2.friend_code.clone())
        .send_request(&app)
        .await;

    assert!(resp.status().is_client_error(), "Re-adding friend fails");

    let resp = TestRequest::post()
        .peer_addr(addr)
        .uri("/friends/add")
        .insert_header(("authorization", "Bearer ".to_owned() + &f1.auth_token))
        .set_payload(f1.friend_code.clone())
        .send_request(&app)
        .await;

    assert!(resp.status().is_client_error(), "Adding self fails");

    let resp = request_auth!(
        app,
        addr,
        get,
        &format!("/users/{}/activity/data", &f1.username),
        f2.auth_token
    );
    assert!(
        resp.status().is_success(),
        "Friends can see eachothers data"
    );

    let resp = request_auth!(app, addr, get, "/friends/list", f2.auth_token);
    assert!(resp.status().is_success(), "Getting friends-list works");

    let friends: Vec<serde_json::Value> = test::read_body_json(resp).await;
    assert_eq!(friends.len(), 1, "Friend appears in friends-list");

    let resp = request!(app, addr, delete, "/users/@me/delete", f1_body);
    assert!(resp.status().is_success(), "Failed to delete user");

    let resp = request!(app, addr, delete, "/users/@me/delete", f2_body);
    assert!(resp.status().is_success(), "Failed to delete user");
}

// TODO: write tests for /friends/regenerate and /friends/remove
