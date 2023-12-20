use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use actix_web::test::{self, TestRequest};
use serde_json::json;

use super::{macros::*, *};
use crate::{
    api::leaderboards::{LeaderboardInvite, LeaderboardName},
    models::{NewUserIdentity, PrivateLeaderboard, SecuredAccessTokenResponse},
};

#[actix_web::test]
async fn creation_joining_and_deletion() {
    let app = test::init_service(App::new().configure(init_test_services)).await;

    let owner_body = json!({"username": "leaderboardowner", "password": "password"});
    let member_body = json!({"username": "leaderboardmember", "password": "password"});
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 80u16);
    let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 80u16);

    let resp = request!(app, addr, post, "/auth/register", owner_body);
    assert!(resp.status().is_success(), "Creating user failed");

    let owner: NewUserIdentity = test::read_body_json(resp).await;

    let resp = request!(app, addr2, post, "/auth/register", member_body);
    assert!(resp.status().is_success(), "Creating user failed");

    let member: NewUserIdentity = test::read_body_json(resp).await;

    let create = LeaderboardName {
        name: "board".to_string(),
    };

    let resp = request_auth!(
        app,
        addr,
        post,
        "/leaderboards/create",
        owner.auth_token,
        create
    );

    assert!(resp.status().is_success(), "Leaderboard creation failed");

    let created: serde_json::Value = test::read_body_json(resp).await;

    let resp = request_auth!(
        app,
        addr,
        post,
        "/leaderboards/create",
        owner.auth_token,
        create
    );

    assert!(
        resp.status().is_client_error(),
        "Duplicate leaderboards cannot exist"
    );

    let invite = LeaderboardInvite {
        invite: created["invite_code"].as_str().unwrap().to_string(),
    };

    let resp = request_auth!(
        app,
        addr,
        post,
        "/leaderboards/join",
        member.auth_token,
        invite
    );

    assert!(resp.status().is_success(), "Joining leaderboard failed");

    let resp = request_auth!(
        app,
        addr,
        post,
        "/leaderboards/join",
        owner.auth_token,
        invite
    );

    assert!(
        resp.status().is_client_error(),
        "Trying to re-join a leaderboard should fail"
    );

    let resp = request_auth!(app, addr, get, "/leaderboards/board", member.auth_token);

    assert!(resp.status().is_success(), "Getting leaderboard failed");

    let board: PrivateLeaderboard = test::read_body_json(resp).await;
    assert_eq!(
        board.members.len(),
        2,
        "Leaderboard member count should be 2"
    );

    let resp = request!(app, addr, post, "/auth/securedaccess", owner_body);
    assert!(
        resp.status().is_success(),
        "Getting secured access token failed"
    );
    let sat: SecuredAccessTokenResponse = test::read_body_json(resp).await;

    let resp = request_auth!(app, addr, delete, "/leaderboards/board", sat.token);
    assert!(resp.status().is_success(), "Leaderboards deletion failed");

    let resp = request_auth!(app, addr, get, "/leaderboards/board", member.auth_token);

    assert!(
        resp.status().is_client_error(),
        "Leaderboard should be deleted"
    );

    let resp = request!(app, addr, delete, "/users/@me/delete", owner_body);
    assert!(resp.status().is_success(), "Failed to delete user");

    let resp = request!(app, addr, delete, "/users/@me/delete", member_body);
    assert!(resp.status().is_success(), "Failed to delete user");
}

// TODO: add tests for all the leaderboards endpoints
