use std::{collections::HashMap, sync::LazyLock};

use actix_web::{
    cookie::Cookie,
    error::*,
    web::{block, Data, Query},
    HttpResponse, Responder,
};
use awc::Client;
use serde_derive::Deserialize;

use crate::{database::Database, error::TimeError};

#[derive(Deserialize)]
struct TokenExchangeRequest {
    code: String,
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    token: String,
}

#[derive(Deserialize)]
struct ClientInfo {
    #[serde(rename = "client_id")]
    id: String,
    #[serde(rename = "client_secret")]
    secret: String,
    redirect_uri: String,
}

#[derive(Deserialize, Debug)]
struct TestausIdApiUser {
    id: String,
    name: String,
    platform: TestausIdPlatformInfo,
}

#[derive(Deserialize, Debug)]
struct TestausIdPlatformInfo {
    id: String,
}

static CLIENT_INFO: LazyLock<ClientInfo> = LazyLock::new(|| {
    toml::from_str(&std::fs::read_to_string("settings.toml").expect("Missing settings.toml"))
        .expect("Invalid Toml in settings.toml")
});

#[get("/auth/callback")]
async fn callback(
    request: Query<TokenExchangeRequest>,
    client: Data<Client>,
    db: Data<Database>,
) -> Result<impl Responder, TimeError> {
    if request.code.chars().any(|c| !c.is_alphanumeric()) {
        return Err(TimeError::BadCode);
    }

    let res = client
        .post("http://id.testausserveri.fi/api/v1/token")
        .insert_header(("content-type", "application/x-www-form-urlencoded"))
        .send_form(&HashMap::from([
            ("code", &request.code),
            ("redirect_uri", &CLIENT_INFO.redirect_uri),
            ("client_id", &CLIENT_INFO.id),
            ("client_secret", &CLIENT_INFO.secret),
        ]))
        .await
        .unwrap()
        .json::<TokenResponse>()
        .await
        .unwrap();

    let res = client
        .get("http://id.testausserveri.fi/api/v1/me")
        .insert_header(("Authorization", format!("Bearer {}", res.token)))
        .send()
        .await
        .unwrap()
        .json::<TestausIdApiUser>()
        .await
        .unwrap();

    let token =
        block(move || db.get()?.testausid_login(res.id, res.name, res.platform.id)).await??;

    Ok(HttpResponse::PermanentRedirect()
        .insert_header(("location", "https://testaustime.fi/oauth_redirect"))
        .cookie(
            Cookie::build("testaustime_token", token)
                .domain("testaustime.fi")
                .path("/")
                .secure(true)
                .finish(),
        )
        .finish())
}
