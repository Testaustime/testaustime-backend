use std::{
    collections::HashMap,
    rc::Rc,
    time::{Duration, Instant},
};

use actix::prelude::*;
use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{HeaderName, HeaderValue},
    Error, HttpResponse,
};
use futures_util::{future::LocalBoxFuture, stream::once};

pub struct RateLimitInfo {
    pub request_count: usize,
    pub last_reset: Instant,
}

pub struct RateLimiterStorage {
    pub clients: HashMap<String, RateLimitInfo>,
    pub maxrpm: usize,
    pub reset_interval: usize,
    event_count: usize,
}

impl Actor for RateLimiterStorage {
    type Context = Context<Self>;
}

impl RateLimiterStorage {
    pub fn new(maxrpm: usize, reset_interval: usize) -> Self {
        RateLimiterStorage {
            clients: HashMap::new(),
            maxrpm,
            reset_interval,
            event_count: 0,
        }
    }
}

struct ConfigRequest;

impl Message for ConfigRequest {
    type Result = Result<(usize, usize), std::io::Error>;
}

impl Handler<ConfigRequest> for RateLimiterStorage {
    type Result = Result<(usize, usize), std::io::Error>;

    fn handle(&mut self, _: ConfigRequest, _: &mut Context<Self>) -> Self::Result {
        Ok((self.maxrpm.to_owned(), self.reset_interval.to_owned()))
    }
}

#[derive(Message)]
#[rtype(result = "()")]
struct ClearRequest;

impl Handler<ClearRequest> for RateLimiterStorage {
    type Result = ();

    fn handle(&mut self, _: ClearRequest, _: &mut Context<Self>) {
        let cur_time = Instant::now();
        self.clients
            .retain(|_, i| cur_time.duration_since(i.last_reset) < Duration::from_secs(1800));
    }
}

struct IpRequest {
    pub ip: String,
}

impl Message for IpRequest {
    type Result = Result<(Option<usize>, Duration), std::io::Error>;
}

impl Handler<IpRequest> for RateLimiterStorage {
    type Result = Result<(Option<usize>, Duration), std::io::Error>;

    fn handle(&mut self, req: IpRequest, ctx: &mut Context<Self>) -> Self::Result {
        if self.event_count > 1000 {
            ctx.add_message_stream(once(async { ClearRequest }));
            self.event_count = 0;
        } else {
            self.event_count += 1;
        };
        if let Some(rlinfo) = self.clients.get_mut(&req.ip) {
            let time = Instant::now();
            let duration = (rlinfo.last_reset).duration_since(time);
            if duration == Duration::from_secs(0) {
                rlinfo.request_count = 1;
                rlinfo.last_reset = time + Duration::from_secs(self.reset_interval as u64);
                Ok((
                    Some(self.maxrpm - rlinfo.request_count),
                    Duration::from_secs(self.reset_interval as u64),
                ))
            } else if rlinfo.request_count as usize >= self.maxrpm {
                Ok((None, duration))
            } else {
                rlinfo.request_count += 1;
                Ok((Some(self.maxrpm - rlinfo.request_count), duration))
            }
        } else {
            self.clients.insert(
                req.ip,
                RateLimitInfo {
                    request_count: 1,
                    last_reset: std::time::Instant::now()
                        + Duration::from_secs(self.reset_interval as u64),
                },
            );
            Ok((
                Some(self.maxrpm - 1),
                Duration::from_secs(self.reset_interval as u64),
            ))
        }
    }
}

pub struct RateLimiter {
    pub storage: Addr<RateLimiterStorage>,
    pub use_peer_addr: bool,
    pub maxrpm: usize,
    pub bypass_token: String,
    pub reset_interval: usize,
}

impl<S, B> Transform<S, ServiceRequest> for RateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimiterTransform<S>;
    type Future = LocalBoxFuture<'static, Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        let ratelimiter = RateLimiterTransform {
            service: Rc::new(service),
            ratelimiter: self.storage.clone(),
            use_peer_addr: self.use_peer_addr,
            bypass_token: self.bypass_token.clone(),
            maxrpm: self.maxrpm,
            reset_interval: self.reset_interval,
        };

        Box::pin(async { Ok(ratelimiter) })
    }
}

pub struct RateLimiterTransform<S> {
    pub service: Rc<S>,
    pub ratelimiter: Addr<RateLimiterStorage>,
    pub use_peer_addr: bool,
    pub maxrpm: usize,
    pub bypass_token: String,
    pub reset_interval: usize,
}

impl<S, B> Service<ServiceRequest> for RateLimiterTransform<S>
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

            if bypass {
                req.headers()
                    .get("client-ip")
                    .and_then(|ip| ip.to_str().ok())
            } else if self.use_peer_addr {
                conn_info.peer_addr()
            } else {
                conn_info.realip_remote_addr()
            }
        } {
            let res = self.ratelimiter.send(IpRequest { ip: ip.to_owned() });
            let service = Rc::clone(&self.service);
            let maxrpm = self.maxrpm;

            Box::pin(async move {
                let (remaining, reset) = res
                    .await
                    .map_err(|e| actix_web::error::ErrorInternalServerError(e))??;
                if let Some(remaining) = remaining {
                    let mut resp = service.call(req).await?;
                    let headers = resp.headers_mut();
                    headers.insert(
                        HeaderName::from_static("ratelimit-limit"),
                        HeaderValue::from_str(&maxrpm.to_string()).unwrap(),
                    );
                    headers.insert(
                        HeaderName::from_static("ratelimit-remaining"),
                        HeaderValue::from_str(&remaining.to_string()).unwrap(),
                    );
                    headers.insert(
                        HeaderName::from_static("ratelimit-reset"),
                        HeaderValue::from_str(&reset.as_secs().to_string()).unwrap(),
                    );
                    Ok(resp.map_into_left_body())
                } else {
                    let response = HttpResponse::TooManyRequests()
                        .insert_header(("ratelimit-limit", maxrpm.to_string()))
                        .insert_header(("ratelimit-remaining", 0usize.to_string()))
                        .insert_header(("ratelimit-reset", reset.as_secs().to_string()))
                        .finish();
                    Ok(req.into_response(response.map_into_right_body()))
                }
            })
        } else {
            Box::pin(async move { Err(actix_web::error::ErrorInternalServerError("wtf")) })
        }
    }
}
