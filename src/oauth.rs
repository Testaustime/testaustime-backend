use oauth2::{
    basic::BasicClient, reqwest::async_http_client, AuthorizationCode, CsrfToken, Scope,
    TokenResponse,
};
use rocket::{
    http::{Cookie, CookieJar, Status},
    response::Redirect,
    State,
};
use serde::{ Deserialize, Serialize };

use crate::database::Database;

#[get("/discord")]
pub async fn authorize(client: &State<BasicClient>) -> Redirect {
    let (auth_url, _) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();

    Redirect::to(String::from(auth_url.as_str()))
}

#[derive(Deserialize, Debug)]
struct DiscordUser {
    pub id: String
}

#[get("/discord/callback?<code>")]
pub async fn callback(
    client: &State<BasicClient>,
    code: String,
    cookies: &CookieJar<'_>,
    database: &State<Database>,
) -> Result<String, Status> {
    let token_result = client
        .exchange_code(AuthorizationCode::new(code))
        .request_async(async_http_client)
        .await
        .unwrap();

    let http_client = reqwest::Client::new();
    let res = http_client
        .get("https://discord.com/api/users/@me")
        .bearer_auth(token_result.access_token().secret())
        .send()
        .await
        .unwrap()
        .json::<DiscordUser>()
        .await
        .unwrap();
    if let Some(user) = database.get_user_by_discord_id(res.id.parse::<u64>().unwrap()).await.unwrap() {
    } else {
        database.new_user(res.id.parse::<u64>().unwrap()).await.unwrap();
    }

    cookies.add_private(Cookie::new("message", "balls"));
    Ok(res.id)
}
