macro_rules! request {
    ($app:ident, $method:tt, $uri:expr) => {
        tower::Service::call(
            ServiceExt::<Request<Body>>::ready(&mut $app).await.unwrap(),
            Request::builder()
                .uri($uri)
                .method(http::Method::$method)
                .extension(axum::extract::ConnectInfo(TEST_ADDR))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    };
    ($app:ident, $method:tt, $uri:expr, $body:expr) => {
        tower::Service::call(
            ServiceExt::<Request<Body>>::ready(&mut $app).await.unwrap(),
            Request::builder()
                .uri($uri)
                .method(http::Method::$method)
                .header(http::header::CONTENT_TYPE, mime::APPLICATION_JSON.as_ref())
                .extension(axum::extract::ConnectInfo(TEST_ADDR))
                .body(Body::from(serde_json::to_vec(&$body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
    };
}

macro_rules! request_auth {
    ($app:expr, $method:tt, $uri:expr, $token:expr) => {
        tower::Service::call(
            ServiceExt::<Request<Body>>::ready(&mut $app).await.unwrap(),
            Request::builder()
                .uri($uri)
                .method(http::Method::$method)
                .header("authorization", "Bearer ".to_owned() + &$token)
                .extension(axum::extract::ConnectInfo(TEST_ADDR))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    };
    ($app:expr, $method:tt, $uri:expr, $token:expr, $body:expr) => {
        tower::Service::call(
            ServiceExt::<Request<Body>>::ready(&mut $app).await.unwrap(),
            Request::builder()
                .uri($uri)
                .method(http::Method::$method)
                .header(http::header::CONTENT_TYPE, "application/json")
                .header("authorization", "Bearer ".to_owned() + &$token)
                .extension(axum::extract::ConnectInfo(TEST_ADDR))
                .body(Body::from(serde_json::to_vec(&$body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
    };
}

pub(crate) use request;
pub(crate) use request_auth;
