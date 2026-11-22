use crate::conf;
use crate::signup::rules::is_tag_topic;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use futures::future;
use hyper::header::HeaderValue;
use hyper::rt::{Future, Stream};
use hyper::{Body, Client, Method, Request};
use hyper_rustls::HttpsConnector;
use serde_json::Value;
use tokio;
use urlencoding::encode;

#[derive(Clone, Copy)]
pub struct ProcessedMove {
    pub notify_stream: &'static str,
    pub notify_topic: &'static str,
    pub tag_topic_prefix: &'static str,
    pub processed_topic: &'static str,
}

impl ProcessedMove {
    pub fn from_conf() -> Self {
        ProcessedMove {
            notify_stream: conf::ZULIP_NOTIFY_STREAM,
            notify_topic: conf::ZULIP_NOTIFY_TOPIC,
            tag_topic_prefix: conf::ZULIP_NOTIFY_TOPIC_PREFIX,
            processed_topic: conf::ZULIP_NOTIFY_PROCESSED_TOPIC,
        }
    }
}

pub fn post_message(
    text: String,
    bot_id: &'static str,
    token: &'static str,
    stream: &'static str,
    topic: impl Into<String>,
    zulip_url: &'static str,
) {
    let topic: String = topic.into();
    tokio::spawn(future::lazy(move || {
        let https = HttpsConnector::new(2);
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
                BASE64.encode(bot_id.to_owned() + ":" + token)
            ))
            .expect("Authorization header value error"),
        );
        *req.body_mut() = format!(
            "type=stream&to={}&subject={}&content={}",
            encode(stream),
            encode(&topic),
            encode(&text)
        )
        .into();

        client
            .request(req)
            .map(|_| {})
            .map_err(|err| println!("Error in post_message: {}", err))
    }));
}

fn basic_auth(bot_id: &str, token: &str) -> HeaderValue {
    HeaderValue::from_str(&format!(
        "Basic {}",
        BASE64.encode(bot_id.to_owned() + ":" + token)
    ))
    .expect("Authorization header value error")
}

pub fn should_move_to_processed(message: &Value, bot_id: &str, cfg: &ProcessedMove) -> bool {
    let str_field = |key: &str| message.get(key).and_then(|v| v.as_str()).unwrap_or("");
    if str_field("type") != "stream" || str_field("sender_email") != bot_id {
        return false;
    }

    let stream_id = message
        .get("stream_id")
        .and_then(|v| v.as_i64())
        .map(|id| id.to_string());
    let in_notify_stream = str_field("display_recipient") == cfg.notify_stream
        || stream_id.as_deref() == Some(cfg.notify_stream);

    let topic = str_field("subject");
    in_notify_stream
        && topic != cfg.processed_topic
        && (topic == cfg.notify_topic || is_tag_topic(topic, cfg.tag_topic_prefix))
}

pub fn move_to_processed_topic_if_eligible(
    message_id: i64,
    bot_id: &'static str,
    token: &'static str,
    zulip_url: &'static str,
    cfg: ProcessedMove,
) {
    tokio::spawn(future::lazy(move || {
        let https = HttpsConnector::new(2);
        let client = Client::builder().build::<_, Body>(https);
        let patch_client = client.clone();
        let message_url = format!("https://{}/api/v1/messages/{}", zulip_url, message_id);

        let mut get_req = Request::new(Body::from(""));
        *get_req.uri_mut() = message_url.parse().unwrap();
        get_req
            .headers_mut()
            .insert(hyper::header::AUTHORIZATION, basic_auth(bot_id, token));

        client
            .request(get_req)
            .and_then(|res| res.into_body().concat2())
            .map_err(|err| println!("Error fetching Zulip message: {}", err))
            .and_then(move |body| {
                let eligible = serde_json::from_slice::<Value>(&body)
                    .map(|resp| should_move_to_processed(&resp["message"], bot_id, &cfg))
                    .unwrap_or(false);
                if !eligible {
                    return future::Either::A(future::ok(()));
                }

                let mut patch_req = Request::new(Body::from(format!(
                    "topic={}&propagate_mode=change_one\
                     &send_notification_to_old_thread=false\
                     &send_notification_to_new_thread=false",
                    encode(cfg.processed_topic)
                )));
                *patch_req.uri_mut() = message_url.parse().unwrap();
                *patch_req.method_mut() = Method::PATCH;
                patch_req.headers_mut().insert(
                    hyper::header::CONTENT_TYPE,
                    HeaderValue::from_static("application/x-www-form-urlencoded"),
                );
                patch_req
                    .headers_mut()
                    .insert(hyper::header::AUTHORIZATION, basic_auth(bot_id, token));

                future::Either::B(
                    patch_client
                        .request(patch_req)
                        .and_then(|res| {
                            let status = res.status();
                            res.into_body().concat2().map(move |body| (status, body))
                        })
                        .map(|(status, body)| {
                            if !status.is_success() {
                                println!(
                                    "Error moving Zulip message: {} {}",
                                    status,
                                    String::from_utf8_lossy(&body)
                                );
                            }
                        })
                        .map_err(|err| println!("Error moving Zulip message: {}", err)),
                )
            })
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const CFG: ProcessedMove = ProcessedMove {
        notify_stream: "mod-notify",
        notify_topic: "default",
        tag_topic_prefix: "Watcher",
        processed_topic: "Watcher processed",
    };

    fn message(sender: &str, stream: &str, topic: &str) -> Value {
        json!({
            "type": "stream",
            "sender_email": sender,
            "stream_id": 42,
            "display_recipient": stream,
            "subject": topic,
        })
    }

    fn moves(sender: &str, stream: &str, topic: &str, cfg: &ProcessedMove) -> bool {
        should_move_to_processed(&message(sender, stream, topic), "bot@z", cfg)
    }

    #[test]
    fn moves_bot_messages_from_notify_topics() {
        assert!(moves("bot@z", "mod-notify", "default", &CFG));
        assert!(moves("bot@z", "mod-notify", "Watcher: Ru", &CFG));
        let by_id = ProcessedMove {
            notify_stream: "42",
            ..CFG
        };
        assert!(moves("bot@z", "mod-notify", "default", &by_id));
    }

    #[test]
    fn ignores_other_messages() {
        assert!(!moves("mod@z", "mod-notify", "default", &CFG));
        assert!(!moves("bot@z", "other", "default", &CFG));
        assert!(!moves("bot@z", "mod-notify", "commands", &CFG));
        assert!(!moves("bot@z", "mod-notify", "Watcher processed", &CFG));
        assert!(!should_move_to_processed(&Value::Null, "bot@z", &CFG));
    }
}
