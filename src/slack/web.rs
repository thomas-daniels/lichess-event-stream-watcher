use futures::future;
use hyper::header::HeaderValue;
use hyper::rt::Future;
use hyper::{Body, Client, Method, Request};
use hyper_tls::HttpsConnector;
use tokio;

pub fn post_message(text: String, token: &'static str, channel: &'static str) {
    tokio::spawn(future::lazy(move || {
        let https = HttpsConnector::new(2).unwrap();
        let client = Client::builder().build::<_, Body>(https);

        let content = json!({
            "channel": channel,
            "text": text
        })
        .to_string();

        let mut req = Request::new(Body::from(content));

        *req.uri_mut() = "https://slack.com/api/chat.postMessage"
            .to_owned()
            .parse()
            .unwrap();
        *req.method_mut() = Method::POST;

        req.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );

        req.headers_mut().insert(
            hyper::header::AUTHORIZATION,
            HeaderValue::from_str(&("Bearer ".to_owned() + token)).unwrap(),
        );

        client
            .request(req)
            .map(|_| {})
            .map_err(|err| println!("Error in post_message: {}", err))
    }));
}
