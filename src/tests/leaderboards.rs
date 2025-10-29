use serde_json::json;

use super::{macros::*, *};
use crate::{
    api::leaderboards::{LeaderboardInvite, LeaderboardName},
    models::{NewUserIdentity, PrivateLeaderboard},
};

#[tokio::test]
async fn creation_joining_and_deletion() {
    let mut app = create_test_router().into_service();

    let owner_body = json!({"username": "leaderboardowner", "password": "password"});
    let member_body = json!({"username": "leaderboardmember", "password": "password"});

    let resp = request!(app, POST, "/auth/register", owner_body);
    assert!(resp.status().is_success(), "Creating user failed");

    let owner: NewUserIdentity = body_to_json(resp).await;

    let resp = request!(app, POST, "/auth/register", member_body);
    assert!(resp.status().is_success(), "Creating user failed");

    let member: NewUserIdentity = body_to_json(resp).await;

    let create = LeaderboardName {
        name: "board".to_string(),
    };

    let resp = request_auth!(app, POST, "/leaderboards/create", owner.auth_token, create);

    assert!(resp.status().is_success(), "Leaderboard creation failed");

    let created: serde_json::Value = body_to_json(resp).await;

    let resp = request_auth!(app, POST, "/leaderboards/create", owner.auth_token, create);

    assert!(
        resp.status().is_client_error(),
        "Duplicate leaderboards cannot exist"
    );

    let invite = LeaderboardInvite {
        invite: created["invite_code"].as_str().unwrap().to_string(),
    };

    let resp = request_auth!(app, POST, "/leaderboards/join", member.auth_token, invite);

    assert!(resp.status().is_success(), "Joining leaderboard failed");

    let resp = request_auth!(app, POST, "/leaderboards/join", owner.auth_token, invite);

    assert!(
        resp.status().is_client_error(),
        "Trying to re-join a leaderboard should fail"
    );

    let resp = request_auth!(app, GET, "/leaderboards/board", member.auth_token);

    assert!(resp.status().is_success(), "Getting leaderboard failed");

    let board: PrivateLeaderboard = body_to_json(resp).await;
    assert_eq!(
        board.members.len(),
        2,
        "Leaderboard member count should be 2"
    );

    let resp = request_auth!(app, DELETE, "/leaderboards/board", owner.auth_token);
    assert!(resp.status().is_success(), "Leaderboards deletion failed");

    let resp = request_auth!(app, GET, "/leaderboards/board", member.auth_token);

    assert!(
        resp.status().is_client_error(),
        "Leaderboard should be deleted"
    );

    let resp = request!(app, DELETE, "/users/@me/delete", owner_body);
    assert!(resp.status().is_success(), "Failed to delete user");

    let resp = request!(app, DELETE, "/users/@me/delete", member_body);
    assert!(resp.status().is_success(), "Failed to delete user");
}

// TODO: add tests for all the leaderboards endpoints
