use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{AuthorizationCode, CsrfToken, Scope};
use oauth2::TokenResponse;
use rocket::response::Redirect;
use serde_json::Value;
use rocket::State;

#[get("/discord")]
pub fn authorize(client: &State<BasicClient>) -> Redirect {
    let (auth_url, _) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();

    Redirect::to(String::from(auth_url.as_str()))
}

#[get("/discord/callback?<code>")]
pub async fn callback(client: &State<BasicClient>, code: String) -> String {
    let token_result = client
        .exchange_code(AuthorizationCode::new(code))
        .request_async(async_http_client)
        .await
        .unwrap();

    let http_client = reqwest::Client::new();
    let res = http_client.get("https://discord.com/api/users/@me")
        .bearer_auth(token_result.access_token().secret())
        .send()
        .await
        .unwrap()
        .json::<Value>()
        .await
        .unwrap();

    res["id"].to_string()
}
