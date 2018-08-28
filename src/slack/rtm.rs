use hyper::header::HeaderValue;
use hyper::rt::{Future, Stream};
use hyper::{Body, Client, Request};
use hyper_tls::HttpsConnector;
use serde_json;

pub fn connect_to_slack(token: &'static str) -> impl Future<Item = (), Error = ()> {
    let https = HttpsConnector::new(2).unwrap();
    let client = Client::builder().build::<_, Body>(https);

    let mut req = Request::new(Body::from(""));
    *req.uri_mut() = ("https://slack.com/api/rtm.connect?token=".to_owned() + token)
        .parse()
        .unwrap();

    req.headers_mut().insert(
        hyper::header::CONTENT_TYPE,
        hyper::header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );

    client
        .request(req)
        .and_then(|res| {
            println!("{}", res.status());
            res.into_body().concat2()
        }).and_then(|body| {
            let resp: serde_json::Value = serde_json::from_slice(&body)
                .expect("could not deserialize Chunk in connect_to_slack");
            println!("{}", resp["url"]);
            // TODO: connect to websocket
            Ok(())
        }).map_err(|err| {
            println!("Error on connect_to_slack: {}", err);
        })
}
