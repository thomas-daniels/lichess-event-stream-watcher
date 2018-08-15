use futures::future;
use hyper::header::HeaderValue;
use hyper::rt::{Future, Stream};
use hyper::{Body, Client, Request};
use hyper_tls::HttpsConnector;
use std::io::{self, Write};

pub fn watch_event_stream(token: &'static str) {
    tokio::run(future::lazy(move || {
        let https = HttpsConnector::new(4).unwrap();
        let client = Client::builder().build::<_, Body>(https);

        let mut req = Request::new(Body::from(""));
        *req.uri_mut() = "https://lichess.org/api/stream/mod".parse().unwrap();

        let bearer = "Bearer ".to_owned() + token;

        req.headers_mut().insert(
            hyper::header::AUTHORIZATION,
            HeaderValue::from_str(&bearer).unwrap(),
        );

        client
            .request(req)
            .and_then(|res| {
                res.into_body().for_each(|chunk| {
                    io::stdout()
                        .write_all(&chunk)
                        .map_err(|e| panic!("Error on for_each chunk: {}", e))
                })
            }).map_err(|err| {
                println!("Error on get: {}", err);
            })
    }));
}
