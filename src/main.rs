#![feature(extern_prelude)]

extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
extern crate futures;

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

mod eventstream;
mod token;
mod signup;

fn main() {
    eventstream::watch_event_stream(token::TOKEN);
}
