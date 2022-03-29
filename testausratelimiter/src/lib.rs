use std::{
    collections::HashMap,
    future::{ready, Ready},
};

use actix::prelude::*;
use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error,
};
use chrono::prelude::*;
use futures_util::{future::LocalBoxFuture, stream::once};

pub struct RateLimiterStorage {
    pub clients: HashMap<String, (i16, NaiveDateTime)>,
    pub maxrpm: usize,
    event_count: usize,
}

pub struct RateLimiter {
    pub storage: Addr<RateLimiterStorage>,
    pub use_peer_addr: bool,
    pub maxrpm: usize,
}

pub struct RateLimiterTransform<S> {
    pub service: S,
    pub ratelimiter: Addr<RateLimiterStorage>,
    pub use_peer_addr: bool,
    pub maxrpm: usize,
}

#[derive(Message)]
#[rtype(result = "usize")]
struct RpmLimitRequest;

#[derive(Message)]
#[rtype(result = "()")]
struct ClearRequest;

struct IpRequest {
    pub ip: String,
}

impl Actor for RateLimiterStorage {
    type Context = Context<Self>;
}

impl RateLimiterStorage {
    pub fn new(max: usize) -> Self {
        RateLimiterStorage {
            clients: HashMap::new(),
            maxrpm: max,
            event_count: 0,
        }
    }
}

impl Message for IpRequest {
    type Result = Result<bool, std::io::Error>;
}

impl Handler<RpmLimitRequest> for RateLimiterStorage {
    type Result = usize;

    fn handle(&mut self, _: RpmLimitRequest, _: &mut Context<Self>) -> Self::Result {
        self.maxrpm
    }
}

impl Handler<ClearRequest> for RateLimiterStorage {
    type Result = ();

    fn handle(&mut self, _: ClearRequest, _: &mut Context<Self>) {
        let cur_time = Local::now().naive_local();
        self.clients.retain(|_, (_, time)| {
            cur_time.signed_duration_since(*time) < chrono::Duration::minutes(30)
        });
    }
}

impl Handler<IpRequest> for RateLimiterStorage {
    type Result = Result<bool, std::io::Error>;

    fn handle(&mut self, req: IpRequest, ctx: &mut Context<Self>) -> Self::Result {
        if self.event_count > 1000 {
            ctx.add_message_stream(once(async { ClearRequest }));
            self.event_count = 0;
        } else {
            self.event_count += 1;
        };
        if let Some((r, s)) = self.clients.get_mut(&req.ip) {
            let time = Local::now().naive_local(); 
            if time.signed_duration_since(*s) > chrono::Duration::minutes(1) {
                *r = 0;
                *s = time;
            } else if *r as usize > self.maxrpm {
                return Ok(false);
            } else {
                *r += 1
            }
        } else {
            self.clients.insert(req.ip, (1, Local::now().naive_local()));
        }
        Ok(true)
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimiterTransform<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimiterTransform {
            service,
            ratelimiter: self.storage.clone(),
            use_peer_addr: self.use_peer_addr,
            maxrpm: self.maxrpm,
        }))
    }
}

impl<S, B> Service<ServiceRequest> for RateLimiterTransform<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = S::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let conn_info = req.connection_info().clone();
        if let Some(ip) = {
            if self.use_peer_addr {
                conn_info.peer_addr()
            } else {
                conn_info.realip_remote_addr()
            }
        } {
            let res = self.ratelimiter.send(IpRequest { ip: ip.to_owned() });
            let resp = self.service.call(req);
            let maxrpm = self.maxrpm;
            Box::pin(async move {
                let res = res
                    .await
                    .map_err(|e| actix_web::error::ErrorInternalServerError(e))??;
                if res {
                    let resp = resp.await?;
                    Ok(resp)
                } else {
                    Err(actix_web::error::ErrorTooManyRequests(format!(
                        "You have sent more than `{}` requests this minute.",
                        maxrpm
                    )))
                }
            })
        } else {
            Box::pin(async move { Err(actix_web::error::ErrorInternalServerError("wtf")) })
        }
    }
}
