pub mod models;
pub mod oauth;
pub mod schema;

#[macro_use]
extern crate diesel;

#[macro_use]
extern crate rocket;

use oauth2::basic::BasicClient;
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[launch]
fn rocket() -> _ {
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
    rocket::build().mount("/", routes![index]).mount("/auth",routes![oauth::callback,oauth::authorize]).manage(client)
}
