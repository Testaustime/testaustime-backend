use actix_web::{
    error::*, Responder, web::{Query, Data},
};

use awc::Client;
use serde_derive::Deserialize;

use crate::error::TimeError;

#[derive(Deserialize)]
struct TokenExchangeRequest {
    state: String
}

#[post("/oauth/token")]
async fn exchange_code(request: Query<TokenExchangeRequest>, client: Data<Client>) -> Result<impl Responder, TimeError> {
    let body = client.get(format!("id.testausserveri.fi/api/v1/token?state={}", request.state))
        .send()
        .await
        .unwrap()
        .body()
        .await
        .unwrap();
    Ok(body)
}
