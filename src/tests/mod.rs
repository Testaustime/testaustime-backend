use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

// TODO add tests for oauth and improve test coverage
use http_body_util::BodyExt;
mod account;
mod activity;
mod auth;
mod friends;
mod leaderboards;
mod macros;

use axum::{body::Body, extract::connect_info::MockConnectInfo, response::Response, Router};
use http::{Request, StatusCode};
use serde::de::DeserializeOwned;
use tower::ServiceExt;

// NOTE: We would like to use diesels Connection::begin_test_transaction
// But cannot use them because our database uses transactions to implement
// some of the routes and there cannot exists transactions within transactions :'(
use crate::create_router_with_openapi;

const TEST_ADDR: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 3000));

async fn body_to_json<T: DeserializeOwned>(response: Response<Body>) -> T {
    let body = response.into_body().collect().await.unwrap().to_bytes();

    serde_json::from_slice(&body).unwrap()
}

fn create_test_router() -> Router {
    let config = crate::TestaustimeConfig {
        bypass_token: "0000".to_string(),
        ratelimit_by_peer_ip: true,
        max_requests_per_min: 30,
        max_registers_per_hour: 5,
        address: "localhost:3000".to_string(),
        database_url: std::env::var("TEST_DATABASE").expect("TEST_DATABASE not defined"),
        allowed_origin: "".to_string(),
        mail_server: "".to_string(),
        mail_user: "".to_string(),
        mail_password: "".to_string(),
    };

    create_router_with_openapi(&config)
        .0
        .layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 3000))))
}

#[tokio::test]
async fn health() {
    let res = create_test_router()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(res.status(), StatusCode::OK)
}
