use event::{Event, Ip, Username};
use futures::future;
use hyper::header::HeaderValue;
use hyper::rt::{Future, Stream};
use hyper::{Body, Client, Request};
use hyper_tls::HttpsConnector;
use std::sync::mpsc::Sender;

pub fn watch_event_stream(tx: Sender<Event>, token: &'static str, rules_path: &'static str) {
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
            .and_then(move |res| {
                res.into_body().for_each(move |chunk| {
                    let string_chunk = &String::from_utf8(chunk.into_bytes().to_vec())
                        .unwrap_or("invalid chunk bytes".to_string());
                    match Event::from_json(string_chunk) {
                        Ok(event) => tx.send(event).unwrap(),
                        _ => {
                            println!("deserialize error for {}", string_chunk);
                        }
                    };
                    Ok(())
                })
            }).map_err(|err| {
                println!("Error on get: {}", err);
            })
    }));
}
