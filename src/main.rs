#![feature(extern_prelude)]

extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

mod event;
mod eventstream;
mod signup;
mod conf;
mod eventhandler;

use std::thread;
use std::sync::mpsc::channel;

fn main() {
    let (tx, rx) = channel::<event::Event>();

    thread::spawn(move || {
        eventhandler::handle_events(rx, conf::TOKEN, conf::RULES_PATH);
    });

    eventstream::watch_event_stream(tx.clone(), conf::TOKEN, conf::RULES_PATH);
}
