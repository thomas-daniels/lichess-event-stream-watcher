use event::Event;
use futures::future;
use futures::future::Loop;
use hyper::header::HeaderValue;
use hyper::rt::{Future, Stream};
use hyper::{Body, Client, Request};
use hyper_rustls::HttpsConnector;
use status::StatusPing;
use std::sync::mpsc::Sender;
use std::thread;

pub fn watch_event_stream(tx: Sender<Event>, token: &'static str, status_tx: Sender<StatusPing>) {
    tokio::spawn(future::loop_fn((), move |_| {
        let https = HttpsConnector::new(2);
        let client = Client::builder().build::<_, Body>(https);

        let mut req = Request::new(Body::from(""));
        *req.uri_mut() = "https://lichess.org/api/stream/mod".parse().unwrap();

        let bearer = "Bearer ".to_owned() + token;

        req.headers_mut().insert(
            hyper::header::AUTHORIZATION,
            HeaderValue::from_str(&bearer).unwrap(),
        );

        let tx2 = tx.clone();
        let status_tx2 = status_tx.clone();

        let mut count = 0;

        client
            .request(req)
            .and_then(move |res| {
                println!("Event stream connection initialized.");
                res.into_body().for_each(move |chunk| {
                    status_tx2.send(StatusPing::StreamEventReceived).unwrap();
                    tx2.send(Event::InternalStreamEventReceived).unwrap();

                    let string_chunk = &String::from_utf8(chunk.into_bytes().to_vec())
                        .unwrap_or("invalid chunk bytes".to_string());
                    let lines: Vec<&str> = string_chunk.split("\n").collect();
                    for line in &lines {
                        count = count + 1;
                        if count % 400 == 0 {
                            println!("400 done");
                            count = 0;
                        }

                        let trimmed = line.trim();
                        if !trimmed.eq("") {
                            match Event::from_json(line) {
                                Ok(event) => tx2.send(event).unwrap(),
                                _ => {
                                    println!("deserialize error for {}", line);
                                }
                            };
                        }
                    }
                    Ok(())
                })
            })
            .map_err(|err| {
                println!("Error on get: {}", err);
            })
            .and_then(|_| {
                println!("Reconnecting to Lichess event stream in 7 seconds...");
                thread::sleep(std::time::Duration::from_millis(7000));
                Ok(Loop::Continue(()))
            })
    }));
}
