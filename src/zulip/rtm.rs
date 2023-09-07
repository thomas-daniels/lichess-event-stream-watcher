use crate::event::Event;
use crate::status::StatusPing;
use crate::zulip::command::handle_command;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use futures::future;
use futures::future::Loop;
use hyper::header::HeaderValue;
use hyper::rt::{Future, Stream};
use hyper::{Body, Client, Method, Request};
use hyper_rustls::HttpsConnector;
use serde_json;
use std::sync::mpsc::Sender;

pub fn connect_to_zulip(
    zulip_url: &'static str,
    token: &'static str,
    bot_id: &'static str,
    bot_name: &'static str,
    listen_stream: &'static str,
    listen_topic: &'static str,
    tx: Sender<Event>,
    status_tx: Sender<StatusPing>,
) {
    tokio::spawn(future::lazy(move || {
        let https = HttpsConnector::new(2);
        let client = Client::builder().build::<_, Body>(https);

        let mut req = Request::new(Body::from(""));
        *req.uri_mut() = format!("https://{}/api/v1/register", zulip_url)
            .parse()
            .unwrap();

        req.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        req.headers_mut().insert(
            hyper::header::AUTHORIZATION,
            HeaderValue::from_str(&format!(
                "Basic {}",
                BASE64.encode(bot_id.to_owned() + ":" + token)
            ))
            .expect("Authorization header value error"),
        );
        *req.body_mut() = "event_types=[\"message\"]".into();
        *req.method_mut() = Method::POST;

        client
            .request(req)
            .and_then(|res| res.into_body().concat2())
            .map_err(|err| {
                println!("Err in connect_to_zulip: {}", err);
            })
            .and_then(move |body| {
                let resp: serde_json::Value = serde_json::from_slice(&body)
                    .expect("could not deserialize Chunk in connect_to_zulip");
                let queue_id = match &resp["queue_id"] {
                    serde_json::Value::String(s) => s,
                    _ => "",
                }
                .to_owned();

                if queue_id == "" {
                    panic!("could not get queue ID");
                }

                let bot_ping = format!("@**{}** ", bot_name);

                future::loop_fn(-1, move |id| {
                    let mut msg_req = Request::new(Body::from(""));
                    *msg_req.uri_mut() = format!(
                        "https://{}/api/v1/events?queue_id={}&last_event_id={}",
                        zulip_url, queue_id, id
                    )
                    .parse()
                    .unwrap();
                    msg_req.headers_mut().insert(
                        hyper::header::CONTENT_TYPE,
                        HeaderValue::from_static("application/x-www-form-urlencoded"),
                    );
                    msg_req.headers_mut().insert(
                        hyper::header::AUTHORIZATION,
                        HeaderValue::from_str(&format!(
                            "Basic {}",
                            BASE64.encode(bot_id.to_owned() + ":" + token)
                        ))
                        .expect("Authorization header value error"),
                    );

                    let mut new_id = id;
                    let tx2 = tx.clone();
                    let status_tx2 = status_tx.clone();
                    let bot_ping2 = bot_ping.clone();
                    client
                        .request(msg_req)
                        .and_then(|res| res.into_body().concat2())
                        .map_err(|err| {
                            println!("Err in connect_to_zulip: {}", err);
                        })
                        .and_then(move |body| {
                            let events_msg: serde_json::Value = serde_json::from_slice(&body)
                                .expect("could not deserialize events from Zulip");

                            if events_msg.get("result")
                                == Some(&serde_json::Value::String("success".to_owned()))
                            {
                                if let Some(events) =
                                    events_msg.get("events").and_then(|e| e.as_array())
                                {
                                    for event in events {
                                        if let Some(event_id) =
                                            event.get("id").and_then(|i| i.as_i64())
                                        {
                                            new_id = event_id;
                                        }
                                        match event.get("type").and_then(|t| t.as_str()) {
                                            Some("message") => {
                                                let message = &event["message"];
                                                let text = message
                                                    .get("content")
                                                    .and_then(|c| c.as_str())
                                                    .unwrap_or("");

                                                if text.starts_with(&bot_ping2)
                                                    && message.get("display_recipient")
                                                        == Some(&serde_json::Value::String(
                                                            listen_stream.to_owned(),
                                                        ))
                                                    && message.get("subject")
                                                        == Some(&serde_json::Value::String(
                                                            listen_topic.to_owned(),
                                                        ))
                                                {
                                                    let text_reply = match handle_command(
                                                        text[bot_ping2.len()..].to_owned(),
                                                        tx2.clone(),
                                                    ) {
                                                        Ok(s) => s,
                                                        Err(e) => Some(e.message),
                                                    };
                                                    match text_reply {
                                                        Some(reply) => {
                                                            super::web::post_message(
                                                                reply,
                                                                bot_id,
                                                                token,
                                                                listen_stream,
                                                                listen_topic,
                                                                zulip_url,
                                                            );
                                                        }
                                                        _ => {}
                                                    }
                                                }

                                                status_tx2
                                                    .send(StatusPing::ZulipPingReceived)
                                                    .unwrap();
                                            }
                                            Some("heartbeat") => {
                                                status_tx2
                                                    .send(StatusPing::ZulipPingReceived)
                                                    .unwrap();
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            } else {
                                println!("non-success from event queue: {:?}", events_msg);
                            }
                            Ok(Loop::Continue(new_id))
                        })
                })
            })
    }));
}
