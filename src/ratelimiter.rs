use std::{net::IpAddr, rc::Rc, sync::Arc};

use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use governor::{
    clock::DefaultClock, middleware::StateInformationMiddleware,
    state::keyed::DefaultKeyedStateStore, RateLimiter,
};
use http::{header::HeaderName, HeaderValue};

type SharedRateLimiter<Key, M> =
    Arc<RateLimiter<Key, DefaultKeyedStateStore<Key>, DefaultClock, M>>;

pub struct TestaustimeRateLimiter {
    pub limiter: SharedRateLimiter<IpAddr, StateInformationMiddleware>,
    pub use_peer_addr: bool,
    pub bypass_token: String,
}

impl<S, B> Transform<S, ServiceRequest> for TestaustimeRateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = TestaustimeRateLimiterTransform<S>;
    type Future = LocalBoxFuture<'static, Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        let transform = Ok(Self::Transform {
            service: Rc::new(service),
            limiter: Arc::clone(&self.limiter),
            use_peer_addr: self.use_peer_addr,
            bypass_token: self.bypass_token.clone(),
        });

        Box::pin(async move { transform })
    }
}

pub struct TestaustimeRateLimiterTransform<S> {
    service: Rc<S>,
    limiter: SharedRateLimiter<IpAddr, StateInformationMiddleware>,
    use_peer_addr: bool,
    bypass_token: String,
}

impl<S, B> Service<ServiceRequest> for TestaustimeRateLimiterTransform<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = S::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let conn_info = req.connection_info().clone();
        if let Some(ip) = {
            let bypass = req
                .headers()
                .get("bypass-token")
                .is_some_and(|token| token.to_str().is_ok_and(|token| self.bypass_token == token));

            let addr = if bypass {
                req.headers()
                    .get("client-ip")
                    .and_then(|ip| ip.to_str().ok())
            } else if self.use_peer_addr {
                conn_info.peer_addr()
            } else {
                conn_info.realip_remote_addr()
            };

            addr.and_then(|addr| addr.parse::<IpAddr>().ok())
        } {
            match self.limiter.check_key(&ip) {
                Ok(state) => {
                    let res = self.service.call(req);

                    Box::pin(async move {
                        let mut res = res.await?;

                        let headers = res.headers_mut();

                        let quota = state.quota();

                        headers.insert(
                            HeaderName::from_static("ratelimit-limit"),
                            HeaderValue::from_str(&quota.burst_size().to_string())?,
                        );

                        headers.insert(
                            HeaderName::from_static("ratelimit-remaining"),
                            HeaderValue::from_str(&state.remaining_burst_capacity().to_string())?,
                        );

                        headers.insert(
                            HeaderName::from_static("ratelimit-reset"),
                            HeaderValue::from_str(
                                &quota.replenish_interval().as_secs().to_string(),
                            )?,
                        );

                        Ok(res.map_into_left_body())
                    })
                }
                Err(denied) => Box::pin(async move {
                    let response = HttpResponse::TooManyRequests()
                        .insert_header(("ratelimit-limit", denied.quota().burst_size().to_string()))
                        .insert_header(("ratelimit-remaining", "0"))
                        .insert_header((
                            "ratelimit-reset",
                            denied.quota().replenish_interval().as_secs().to_string(),
                        ))
                        .finish();

                    Ok(req.into_response(response.map_into_right_body()))
                }),
            }
        } else {
            Box::pin(async move {
                Err(actix_web::error::ErrorInternalServerError(
                    "Failed to get request ip (?)",
                ))
            })
        }
    }
}
