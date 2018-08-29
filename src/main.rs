#![feature(extern_prelude)]
#![feature(try_trait)]

extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
extern crate tungstenite;
extern crate url;

#[macro_use]
extern crate serde_derive;

extern crate serde;
#[macro_use]
extern crate serde_json;

mod conf;
mod event;
mod eventhandler;
mod eventstream;
mod signup;
mod slack;

use futures::future;
use std::sync::mpsc::channel;

fn main() {
    tokio::run(future::lazy(move || {
        let (tx, rx) = channel::<event::Event>();

        tokio::spawn(eventstream::watch_event_stream(
            tx.clone(),
            conf::TOKEN,
            conf::RULES_PATH,
        ));

        slack::rtm::connect_to_slack(conf::SLACK_BOT_TOKEN, conf::SLACK_BOT_USER_ID, tx.clone());

        eventhandler::handle_events(
            rx,
            conf::TOKEN,
            conf::RULES_PATH,
            conf::SLACK_BOT_TOKEN,
            conf::SLACK_CHANNEL,
        );

        Ok(())
    }));
}
