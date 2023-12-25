pub mod secured_access;

use std::rc::Rc;

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::{ErrorInternalServerError, ErrorUnauthorized},
    web::Data,
    Error, FromRequest, HttpMessage,
};
use futures::future::LocalBoxFuture;

use self::secured_access::SecuredAccessTokenStorage;
use crate::{database::DatabaseWrapper, models::UserIdentity};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Authentication {
    NoAuth,
    AuthToken(UserIdentity),
    SecuredAuthToken(UserIdentity),
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

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let db = DatabaseWrapper::extract(req.request());
        let secured_access_storage = req
            .app_data::<Data<SecuredAccessTokenStorage>>()
            .expect("Secured token access storage not initialized")
            .clone();
        let auth = req.headers().get("Authorization").cloned();
        let service = Rc::clone(&self.service);

        Box::pin(async move {
            if let Some(auth) = auth {
                if let Some(token) = auth.to_str().unwrap().trim().strip_prefix("Bearer ") {
                    let db = db.await.map_err(ErrorInternalServerError)?;

                    if let Ok(secured_access_instance) = secured_access_storage.get(token).clone() {
                        let user = db
                            .get_user_by_id(secured_access_instance.user_id)
                            .await
                            .map_err(ErrorUnauthorized)?;

                        req.extensions_mut()
                            .insert(Authentication::SecuredAuthToken(user));
                    } else {
                        let user = db
                            .get_user_by_token(token.to_string())
                            .await
                            .map_err(ErrorUnauthorized);

                        if let Ok(user_identity) = user {
                            req.extensions_mut()
                                .insert(Authentication::AuthToken(user_identity));
                        } else {
                            req.extensions_mut().insert(Authentication::NoAuth);
                        }
                    }
                } else {
                    req.extensions_mut().insert(Authentication::NoAuth);
                }
            } else {
                req.extensions_mut().insert(Authentication::NoAuth);
            }

            let resp = service.call(req).await?;
            Ok(resp)
        })
    }
}
