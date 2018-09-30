use event::Event;
use eventstream;
use futures::future::{self, loop_fn, Loop};
use futures::Future;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};
use tokio;
use tokio::timer::Delay;

pub enum StatusPing {
    StreamEventReceived,
    EnsureAliveConnection,
}

pub fn status_loop(
    rx: Receiver<StatusPing>,
    main_tx: Sender<Event>,
    token: &'static str,
    status_tx: Sender<StatusPing>,
) {
    tokio::spawn(future::loop_fn(
        Instant::now(),
        move |latest_stream_event| {
            let ping = rx.recv().unwrap();

            match ping {
                StatusPing::StreamEventReceived => Ok(Loop::Continue(Instant::now())),
                StatusPing::EnsureAliveConnection => {
                    if latest_stream_event.elapsed().as_secs() > 90 {
                        eventstream::watch_event_stream(main_tx.clone(), token, status_tx.clone());
                        println!("Event stream watcher restarted.");
                        Ok(Loop::Continue(Instant::now()))
                    } else {
                        Ok(Loop::Continue(latest_stream_event))
                    }
                }
            }
        },
    ));
}

pub fn periodically_ensure_alive_connection(status_tx: Sender<StatusPing>) {
    tokio::spawn(loop_fn((), move |_| {
        let status_tx2 = status_tx.clone();
        Delay::new(Instant::now() + Duration::from_secs(15))
            .and_then(move |_| {
                status_tx2.send(StatusPing::EnsureAliveConnection).unwrap();
                Ok(Loop::Continue(()))
            })
            .map_err(|e| println!("Err in periodically_...: {}", e))
    }));
}
