use std::rc::Rc;

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, web::{Data, block}, HttpMessage, error::ErrorUnauthorized,
};
use futures::future::LocalBoxFuture;

use crate::{database::Database, error::TimeError, models::UserIdentity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Authentication {
    NoAuth,
    AuthToken(UserIdentity),
}

pub struct AuthMiddleware;

pub struct AuthMiddlewareTransform<S> {
    service: Rc<S>,
}

impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddlewareTransform<S>;
    type Future = LocalBoxFuture<'static, Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        Box::pin(async move {
            Ok(AuthMiddlewareTransform {
                service: Rc::new(service),
            })
        })
    }
}

impl<S, B> Service<ServiceRequest> for AuthMiddlewareTransform<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = S::Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let db = req.extract::<Data<Database>>();
        let auth = req.headers().get("Authorization").cloned();
        let service = Rc::clone(&self.service);
        Box::pin(async move {
            if let Some(auth) = auth {
                let db = db.await?;
                let user = block(move || {
                    let Some(token) = auth.to_str().unwrap().trim().strip_prefix("Bearer ") else { return Err(TimeError::Unauthorized) };
                    db.get()?.get_user_by_token(token)
                }).await?.map_err(ErrorUnauthorized)?;

                req.extensions_mut().insert(Authentication::AuthToken(user));
            } else {
                req.extensions_mut().insert(Authentication::NoAuth);
            }
            let resp = service.call(req).await?;

            Ok(resp)
        })
    }
}
