extern crate base64;
extern crate chrono;
extern crate futures;
extern crate hyper;
extern crate hyper_rustls;
extern crate maxminddb;
extern crate rand;
extern crate tokio;
extern crate uaparser;
extern crate url;
extern crate urlencoding;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate serde_regex;

extern crate rlua;

mod conf;
mod event;
mod eventhandler;
mod eventstream;
mod lua;
mod signup;
mod status;
mod zulip;

use futures::future;
use std::sync::mpsc::channel;

fn main() {
    tokio::run(future::lazy(move || {
        let (tx, rx) = channel::<event::Event>();
        let (status_tx, status_rx) = channel::<status::StatusPing>();

        eventstream::watch_event_stream(tx.clone(), conf::TOKEN, status_tx.clone());

        zulip::rtm::connect_to_zulip(
            conf::ZULIP_URL,
            conf::ZULIP_BOT_TOKEN,
            conf::ZULIP_BOT_ID,
            conf::ZULIP_BOT_USERNAME,
            conf::ZULIP_COMMAND_STREAM,
            conf::ZULIP_COMMAND_TOPIC,
            tx.clone(),
            status_tx.clone(),
        );

        status::status_loop(status_rx, tx.clone(), conf::TOKEN, status_tx.clone());
        status::periodically_ensure_alive_connection(status_tx.clone());
        signup::rules::expiry_loop(tx.clone());

        eventhandler::handle_events(
            rx,
            conf::TOKEN,
            conf::RULES_PATH,
            conf::GEOIP_DB_PATH,
            conf::UAP_REGEXES_PATH,
            conf::ZULIP_BOT_ID,
            conf::ZULIP_BOT_TOKEN,
            conf::ZULIP_NOTIFY_STREAM,
            conf::ZULIP_NOTIFY_TOPIC,
            conf::ZULIP_COMMAND_STREAM,
            conf::ZULIP_COMMAND_TOPIC,
            conf::ZULIP_LOG_STREAM,
            conf::ZULIP_LOG_TOPIC,
            conf::ZULIP_URL,
        );

        Ok(())
    }));
}
