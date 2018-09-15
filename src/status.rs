use event::Event;
use eventstream;
use futures::future;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};
use tokio;

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
    tokio::spawn(future::lazy(move || {
        let mut latest_stream_event = Instant::now();

        loop {
            let ping = rx.recv().unwrap();

            match ping {
                StatusPing::StreamEventReceived => latest_stream_event = Instant::now(),
                StatusPing::EnsureAliveConnection => {
                    if latest_stream_event.elapsed().as_secs() > 90 {
                        eventstream::watch_event_stream(main_tx.clone(), token, status_tx.clone());
                        println!("Event stream watcher restarted.");
                        latest_stream_event = Instant::now();
                    }
                }
            }
        }
        #[allow(unreachable_code)]
        Ok(())
    }));
}

pub fn periodically_ensure_alive_connection(status_tx: Sender<StatusPing>) {
    tokio::spawn(future::lazy(move || {
        loop {
            thread::sleep(Duration::from_secs(15));
            status_tx.send(StatusPing::EnsureAliveConnection).unwrap();
        }
        #[allow(unreachable_code)]
        Ok(())
    }));
}
