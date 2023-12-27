// TODO add tests for oauth and improve test coverage
mod account;
mod activity;
mod auth;
mod friends;
mod leaderboards;
mod macros;

use std::{num::NonZeroU32, sync::Arc};

use actix_web::{
    http::StatusCode,
    test, web,
    web::{Data, ServiceConfig},
    App,
};
use governor::{Quota, RateLimiter};

// NOTE: We would like to use diesels Connection::begin_test_transaction
// But cannot use them because our database uses transactions to implement
// some of the routes and there cannot exists transactions within transactions :'(
use crate::database::Database;

// FIXME: There is quite a lot of duplicate code from main
// in this function, perhaps these functions could be unified somehow.
fn init_test_services(cfg: &mut ServiceConfig) {
    let db_url =
        std::env::var("TEST_DATABASE").expect("TEST_DATABASE not set, refusing to run tests");

    let ratelimiter = Arc::new(
        RateLimiter::keyed(Quota::per_minute(NonZeroU32::new(500_u32).unwrap())).with_middleware(),
    );

    let heartbeat_store = Data::new(crate::api::activity::HeartBeatMemoryStore::new());
    let leaderboard_cache = Data::new(crate::api::leaderboards::LeaderboardCache::new());

    let secured_access_token_storage = Data::new(crate::SecuredAccessTokenStorage::new());

    #[cfg(feature = "testausid")]
    let client = crate::Client::new();

    let cors = crate::Cors::default()
        .allow_any_origin()
        .allowed_methods(vec!["GET", "POST", "DELETE"])
        .allowed_headers(vec![
            http::header::AUTHORIZATION,
            http::header::ACCEPT,
            http::header::CONTENT_TYPE,
        ])
        .max_age(3600);
    let query_config = crate::QueryConfig::default().error_handler(|err, _| match err {
        crate::QueryPayloadError::Deserialize(e) => {
            crate::ErrorBadRequest(json!({ "error": e.to_string() }))
        }
        _ => unreachable!(),
    });

    cfg.service(
        web::scope("")
            .app_data(Data::new(crate::RegisterLimiter {
                limit_by_peer_ip: false,
                storage: crate::DashMap::new(),
            }))
            .app_data(Data::new(Database::new(db_url)))
            .app_data(query_config)
            .app_data(Data::clone(&secured_access_token_storage))
            .wrap(cors)
            .service(crate::api::health)
            .service(crate::api::auth::register)
            .service({
                let scope = web::scope("")
                    .wrap(crate::AuthMiddleware)
                    .wrap(crate::TestaustimeRateLimiter {
                        limiter: Arc::clone(&ratelimiter),
                        use_peer_addr: false,
                        bypass_token: String::from("balls"),
                    })
                    .service({
                        web::scope("/activity")
                            .service(crate::api::activity::update)
                            .service(crate::api::activity::delete)
                            .service(crate::api::activity::flush)
                            .service(crate::api::activity::rename_project)
                    })
                    .service(crate::api::auth::login)
                    .service(crate::api::auth::regenerate)
                    .service(crate::api::auth::changeusername)
                    .service(crate::api::auth::changepassword)
                    .service(crate::api::auth::get_secured_access_token)
                    .service(crate::api::account::change_settings)
                    .service(crate::api::friends::add_friend)
                    .service(crate::api::friends::get_friends)
                    .service(crate::api::friends::regenerate_friend_code)
                    .service(crate::api::friends::remove)
                    .service(crate::api::users::my_profile)
                    .service(crate::api::users::get_activities)
                    .service(crate::api::users::get_current_activity)
                    .service(crate::api::users::delete_user)
                    .service(crate::api::users::my_leaderboards)
                    .service(crate::api::users::get_activity_summary)
                    .service(crate::api::leaderboards::create_leaderboard)
                    .service(crate::api::leaderboards::get_leaderboard)
                    .service(crate::api::leaderboards::join_leaderboard)
                    .service(crate::api::leaderboards::leave_leaderboard)
                    .service(crate::api::leaderboards::delete_leaderboard)
                    .service(crate::api::leaderboards::promote_member)
                    .service(crate::api::leaderboards::demote_member)
                    .service(crate::api::leaderboards::kick_member)
                    .service(crate::api::leaderboards::regenerate_invite)
                    .service(crate::api::search::search_public_users)
                    .service(crate::api::stats::stats);
                #[cfg(feature = "testausid")]
                {
                    scope.service(crate::api::oauth::callback)
                }
            }),
    )
    .app_data(Data::clone(&heartbeat_store))
    .app_data(Data::clone(&leaderboard_cache));
    #[cfg(feature = "testausid")]
    {
        cfg.app_data(Data::new(client));
    }
}

#[actix_web::test]
async fn health() {
    let app = test::init_service(App::new().configure(init_test_services)).await;
    let req = test::TestRequest::with_uri("/health").to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), StatusCode::OK)
}
