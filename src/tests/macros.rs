macro_rules! request {
    ($app:expr, $addr:expr, $method:tt, $uri:expr) => {
        TestRequest::$method()
            .peer_addr($addr)
            .uri($uri)
            .send_request(&$app)
            .await
    };
    ($app:expr, $addr:expr, $method:tt, $uri:expr, $body:expr) => {
        TestRequest::$method()
            .peer_addr($addr)
            .uri($uri)
            .set_json(&$body)
            .send_request(&$app)
            .await
    };
}

macro_rules! request_auth {
    ($app:expr, $addr:expr, $method:tt, $uri:expr, $token:expr) => {
        TestRequest::$method()
            .peer_addr($addr)
            .uri($uri)
            .insert_header(("authorization", "Bearer ".to_owned() + &$token))
            .send_request(&$app)
            .await
    };
    ($app:expr, $addr:expr, $method:tt, $uri:expr, $token:expr, $body:expr) => {
        TestRequest::$method()
            .peer_addr($addr)
            .uri($uri)
            .set_json(&$body)
            .insert_header(("authorization", "Bearer ".to_owned() + &$token))
            .send_request(&$app)
            .await
    };
}

pub(crate) use request;
pub(crate) use request_auth;
