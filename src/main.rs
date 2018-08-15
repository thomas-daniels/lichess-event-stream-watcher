#![feature(extern_prelude)]

extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
extern crate futures;

mod eventstream;
mod token;
mod signup;

fn main() {
    eventstream::watch_event_stream(token::TOKEN);
}
