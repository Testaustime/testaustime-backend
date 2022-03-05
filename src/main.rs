#![feature(let_else)]

mod api;
mod database;
pub mod models;
pub mod schema;
mod user;
mod utils;

use actix_web::{
    dev::ServiceRequest, get, middleware::Logger, web, web::Data, App, Error, HttpServer, Responder,
};

#[macro_use]
extern crate actix_web;

#[macro_use]
extern crate log;

#[macro_use]
extern crate diesel;

use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    let client = BasicClient::new(
        ClientId::new(dotenv::var("DISCORD_CLIENT_ID").unwrap()),
        Some(ClientSecret::new(
            dotenv::var("DISCORD_CLIENT_SECRET").unwrap(),
        )),
        AuthUrl::new("https://discord.com/api/oauth2/authorize".to_string()).unwrap(),
        Some(TokenUrl::new("https://discord.com/api/oauth2/token".to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(dotenv::var("DISCORD_REDIRECT_URI").unwrap()).unwrap());

    HttpServer::new(move || {
        let database = database::Database::new();
        let heartbeat_store = api::HeartBeatMemoryStore::default();
        App::new()
            .wrap(Logger::default())
            .service(api::update)
            .service(api::get_activities)
            .app_data(Data::new(database))
            .app_data(Data::new(heartbeat_store))
    })
    .bind("localhost:8000")?
    .run()
    .await
}
