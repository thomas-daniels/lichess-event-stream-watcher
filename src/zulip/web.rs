use futures::future;
use hyper::header::HeaderValue;
use hyper::rt::Future;
use hyper::{Body, Client, Method, Request};
use hyper_tls::HttpsConnector;
use tokio;
use urlencoding::encode;

pub fn post_message(
    text: String,
    bot_id: &'static str,
    token: &'static str,
    stream: &'static str,
    topic: &'static str,
    zulip_url: &'static str,
) {
    tokio::spawn(future::lazy(move || {
        let https = HttpsConnector::new(2).unwrap();
        let client = Client::builder().build::<_, Body>(https);

        let mut req = Request::new(Body::from(""));
        *req.uri_mut() = format!("https://{}/api/v1/messages", zulip_url)
            .parse()
            .unwrap();
        *req.method_mut() = Method::POST;
        req.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        req.headers_mut().insert(
            hyper::header::AUTHORIZATION,
            HeaderValue::from_str(&format!(
                "Basic {}",
                base64::encode(bot_id.to_owned() + ":" + token)
            ))
            .expect("Authorization header value error"),
        );
        *req.body_mut() = format!(
            "type=stream&to={}&subject={}&content={}",
            encode(stream),
            encode(topic),
            encode(&text)
        )
        .into();

        client
            .request(req)
            .map(|_| {})
            .map_err(|err| println!("Error in post_message: {}", err))
    }));
}
