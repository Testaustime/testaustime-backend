use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::{ConnectInfo, Request},
    response::Response,
};
use futures_util::future::BoxFuture;
use governor::{
    clock::DefaultClock, middleware::StateInformationMiddleware,
    state::keyed::DefaultKeyedStateStore, RateLimiter,
};
use http::{
    header::{HeaderName, FORWARDED},
    HeaderValue, StatusCode,
};
use tower::{Layer, Service};

type SharedRateLimiter<Key, M> =
    Arc<RateLimiter<Key, DefaultKeyedStateStore<Key>, DefaultClock, M>>;

#[derive(Clone)]
pub struct TestaustimeRateLimiter {
    pub limiter: SharedRateLimiter<IpAddr, StateInformationMiddleware>,
    pub use_peer_addr: bool,
    pub bypass_token: String,
}

impl<S> Layer<S> for TestaustimeRateLimiter {
    type Service = TestaustimeRateLimiterService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Self::Service {
            inner,
            limiter: Arc::clone(&self.limiter),
            use_peer_addr: self.use_peer_addr,
            bypass_token: self.bypass_token.clone(),
        }
    }
}

#[derive(Clone)]
pub struct TestaustimeRateLimiterService<S> {
    inner: S,
    limiter: SharedRateLimiter<IpAddr, StateInformationMiddleware>,
    use_peer_addr: bool,
    bypass_token: String,
}

impl<S> Service<Request> for TestaustimeRateLimiterService<S>
where
    S: Service<Request, Response = Response>,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        if let Some(ip) = {
            let conn_info = req
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .unwrap()
                .ip();

            let bypass = req
                .headers()
                .get("bypass-token")
                .is_some_and(|token| token.to_str().is_ok_and(|token| self.bypass_token == token));

            let addr = if bypass {
                req.headers()
                    .get("client-ip")
                    .and_then(|ip| ip.to_str().ok().and_then(|ip| ip.parse::<IpAddr>().ok()))
            } else if self.use_peer_addr {
                Some(conn_info)
            } else {
                let header = req
                    .headers()
                    .get("x-forwarded-for")
                    .or_else(|| req.headers().get(FORWARDED));

                Some(
                    header
                        .and_then(|ip| ip.to_str().ok().and_then(|ip| ip.parse::<IpAddr>().ok()))
                        .unwrap_or(conn_info),
                )
            };

            addr
        } {
            match self.limiter.check_key(&ip) {
                Ok(state) => {
                    let res = self.inner.call(req);

                    Box::pin(async move {
                        let mut res = res.await?;

                        let headers = res.headers_mut();

                        let quota = state.quota();

                        headers.insert(
                            HeaderName::from_static("ratelimit-limit"),
                            HeaderValue::from_str(&quota.burst_size().to_string()).unwrap(),
                        );

                        headers.insert(
                            HeaderName::from_static("ratelimit-remaining"),
                            HeaderValue::from_str(&state.remaining_burst_capacity().to_string())
                                .unwrap(),
                        );

                        headers.insert(
                            HeaderName::from_static("ratelimit-reset"),
                            HeaderValue::from_str(
                                &quota.replenish_interval().as_secs().to_string(),
                            )
                            .unwrap(),
                        );

                        Ok(res)
                    })
                }
                Err(denied) => Box::pin(async move {
                    Ok(Response::builder()
                        .status(StatusCode::TOO_MANY_REQUESTS)
                        .header("ratelimit-limit", denied.quota().burst_size().to_string())
                        .header("ratelimit-remaining", "0")
                        .header(
                            "ratelimit-reset",
                            denied.quota().replenish_interval().as_secs().to_string(),
                        )
                        .body(Body::empty())
                        .unwrap())
                }),
            }
        } else {
            Box::pin(async move {
                Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .body(Body::empty())
                    .unwrap())
            })
        }
    }
}
