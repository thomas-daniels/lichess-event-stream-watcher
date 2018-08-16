#![feature(extern_prelude)]

extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

mod eventstream;
mod signup;
mod token;
mod event;

fn main() {
    eventstream::watch_event_stream(token::TOKEN);
}
