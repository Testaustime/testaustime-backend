use std::{collections::HashMap, sync::LazyLock};

use axum::{extract::Query, response::Redirect};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use reqwest::Client;
use serde_derive::Deserialize;

use crate::{database::DatabaseWrapper, error::TimeError};

#[derive(Deserialize)]
pub struct TokenExchangeRequest {
    code: String,
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    token: String,
}

#[cfg(feature = "testausid")]
#[derive(Debug, Deserialize, Clone)]
pub struct ClientInfo {
    #[serde(rename = "client_id")]
    pub id: String,
    #[serde(rename = "client_secret")]
    pub secret: String,
    pub redirect_uri: String,
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

#[utoipa::path(
    get,
    path = "/auth/callback",
    responses(
        (status = OK)
    )
)]
pub async fn callback(
    db: DatabaseWrapper,
    jar: CookieJar,
    request: Query<TokenExchangeRequest>,
) -> Result<(CookieJar, Redirect), TimeError> {
    if request.code.chars().any(|c| !c.is_alphanumeric()) {
        return Err(TimeError::BadCode);
    }

    // Maybe store in state?
    let client = Client::new();

    let res = client
        .post("http://id.testausserveri.fi/api/v1/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .form(&HashMap::from([
            ("code", &request.code),
            ("redirect_uri", &CLIENT_INFO.redirect_uri),
            ("client_id", &CLIENT_INFO.id),
            ("client_secret", &CLIENT_INFO.secret),
        ]))
        .send()
        .await
        .unwrap()
        .json::<TokenResponse>()
        .await
        .unwrap();

    let res = client
        .get("http://id.testausserveri.fi/api/v1/me")
        .header("Authorization", format!("Bearer {}", res.token))
        .send()
        .await
        .unwrap()
        .json::<TestausIdApiUser>()
        .await
        .unwrap();

    let token = db
        .testausid_login(res.id, res.name, res.platform.id)
        .await?;

    Ok((
        jar.add(
            Cookie::build(("testaustime_token", token))
                .domain("testaustime.fi")
                .path("/")
                .secure(true),
        ),
        Redirect::permanent("https://testaustime.fi/oauth_redirect"),
    ))
}
