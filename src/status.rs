use conf;
use event::Event;
use eventstream;
use futures::future::{self, loop_fn, Loop};
use futures::Future;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};
use tokio;
use tokio::timer::Delay;
use zulip;

pub enum StatusPing {
    StreamEventReceived,
    EnsureAliveConnectionLichess,
    EnsureAliveConnectionZulip,
    ZulipPingReceived,
}

pub fn status_loop(
    rx: Receiver<StatusPing>,
    main_tx: Sender<Event>,
    token: &'static str,
    status_tx: Sender<StatusPing>,
) {
    tokio::spawn(future::loop_fn(
        (Instant::now(), Instant::now()),
        move |(latest_stream_event, latest_zulip_event)| {
            let ping = rx.recv().unwrap();

            match ping {
                StatusPing::StreamEventReceived => {
                    Ok(Loop::Continue((Instant::now(), latest_zulip_event)))
                }
                StatusPing::EnsureAliveConnectionLichess => {
                    if latest_stream_event.elapsed().as_secs() > 90 {
                        eventstream::watch_event_stream(main_tx.clone(), token, status_tx.clone());
                        println!("Event stream watcher restarted.");
                        Ok(Loop::Continue((Instant::now(), latest_zulip_event)))
                    } else {
                        Ok(Loop::Continue((latest_stream_event, latest_zulip_event)))
                    }
                }
                StatusPing::ZulipPingReceived => {
                    Ok(Loop::Continue((latest_stream_event, Instant::now())))
                }
                StatusPing::EnsureAliveConnectionZulip => {
                    if latest_zulip_event.elapsed().as_secs() > 720 {
                        zulip::rtm::connect_to_zulip(
                            conf::ZULIP_URL,
                            conf::ZULIP_BOT_TOKEN,
                            conf::ZULIP_BOT_ID,
                            conf::ZULIP_BOT_USERNAME,
                            conf::ZULIP_COMMAND_STREAM,
                            conf::ZULIP_COMMAND_TOPIC,
                            main_tx.clone(),
                            status_tx.clone(),
                        );
                        println!("Slack connection restarted.");
                        Ok(Loop::Continue((latest_stream_event, latest_zulip_event)))
                    } else {
                        Ok(Loop::Continue((latest_stream_event, latest_zulip_event)))
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
                status_tx2
                    .send(StatusPing::EnsureAliveConnectionLichess)
                    .unwrap();
                status_tx2
                    .send(StatusPing::EnsureAliveConnectionZulip)
                    .unwrap();
                Ok(Loop::Continue(()))
            })
            .map_err(|e| println!("Err in periodically_...: {}", e))
    }));
}
