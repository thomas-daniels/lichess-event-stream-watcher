use event::Event;
use futures::future;
use hyper::header::HeaderValue;
use hyper::rt::{Future, Stream};
use hyper::{Body, Client, Request};
use hyper_tls::HttpsConnector;
use serde_json;
use slack::command::handle_command;
use slack::event::{RtmRecv, RtmSend};
use std::sync::mpsc::Sender;
use tungstenite::{connect, Message};
use url::Url;

pub fn connect_to_slack(token: &'static str, bot_id: &'static str, tx: Sender<Event>) {
    tokio::spawn(future::lazy(move || {
        let https = HttpsConnector::new(2).unwrap();
        let client = Client::builder().build::<_, Body>(https);

        let mut req = Request::new(Body::from(""));
        *req.uri_mut() = ("https://slack.com/api/rtm.connect?token=".to_owned() + token)
            .parse()
            .unwrap();

        req.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        client
            .request(req)
            .and_then(|res| res.into_body().concat2())
            .map_err(|err| println!("Err in connect_to_slack: {}", err))
            .and_then(move |body| {
                let resp: serde_json::Value = serde_json::from_slice(&body)
                    .expect("could not deserialize Chunk in connect_to_slack");
                let ws_url = match &resp["url"] {
                    serde_json::Value::String(s) => s,
                    _ => "",
                };

                let (mut socket, _) =
                    connect(Url::parse(ws_url).unwrap()).expect("Cannot connect in rtm_handler");

                let bot_ping = format!("<@{}> ", bot_id);

                let mut id = 0;

                loop {
                    let msg = socket
                        .read_message()
                        .expect("Error reading Slack WebSocket message");
                    println!("Received msg: {}", &msg);
                    match msg {
                        Message::Text(text) => match serde_json::from_str(&text) {
                            Ok(message) => match message {
                                RtmRecv::Message { text, channel, .. } => {
                                    if text.starts_with(&bot_ping) {
                                        id += 1;
                                        let text_reply = match handle_command(
                                            text[bot_ping.len()..].to_owned(),
                                            tx.clone(),
                                        ) {
                                            Ok(s) => s,
                                            Err(_) => Some("Failed to parse command.".to_owned()),
                                        };
                                        match text_reply {
                                            Some(reply) => {
                                                socket
                                                    .write_message(Message::Text(
                                                        serde_json::to_string(&RtmSend {
                                                            id: id,
                                                            type_: "message".to_owned(),
                                                            channel: channel,
                                                            text: reply,
                                                        }).unwrap(),
                                                    )).unwrap();
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            },
                            _ => {}
                        },
                        _ => {}
                    }
                }
                Ok(()) // unreachable code, but required for tokio::spawn
            })
    }));
}
