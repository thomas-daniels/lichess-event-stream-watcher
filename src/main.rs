#![feature(extern_prelude)]

extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

mod conf;
mod event;
mod eventhandler;
mod eventstream;
mod signup;

use std::sync::mpsc::channel;
use std::thread;

fn main() {
    let (tx, rx) = channel::<event::Event>();

    thread::spawn(move || {
        eventhandler::handle_events(rx, conf::TOKEN, conf::RULES_PATH);
    });

    eventstream::watch_event_stream(tx.clone(), conf::TOKEN, conf::RULES_PATH);
}
