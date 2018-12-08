use conf;
use event::Event;
use eventstream;
use futures::future::{self, loop_fn, Loop};
use futures::Future;
use slack;
use std::sync::mpsc::{Receiver, Sender};
use std::time::{Duration, Instant};
use tokio;
use tokio::timer::Delay;

pub enum StatusPing {
    StreamEventReceived,
    EnsureAliveConnectionLichess,
    EnsureAliveConnectionSlack,
    SlackPingReceived,
}

pub fn status_loop(
    rx: Receiver<StatusPing>,
    main_tx: Sender<Event>,
    token: &'static str,
    status_tx: Sender<StatusPing>,
) {
    tokio::spawn(future::loop_fn(
        (Instant::now(), Instant::now()),
        move |(latest_stream_event, latest_slack_event)| {
            let ping = rx.recv().unwrap();

            match ping {
                StatusPing::StreamEventReceived => {
                    Ok(Loop::Continue((Instant::now(), latest_slack_event)))
                }
                StatusPing::EnsureAliveConnectionLichess => {
                    println!("EnsureAliveConnectionLichess received");
                    if latest_stream_event.elapsed().as_secs() > 90 {
                        eventstream::watch_event_stream(main_tx.clone(), token, status_tx.clone());
                        println!("Event stream watcher restarted.");
                        Ok(Loop::Continue((Instant::now(), latest_slack_event)))
                    } else {
                        Ok(Loop::Continue((latest_stream_event, latest_slack_event)))
                    }
                }
                StatusPing::SlackPingReceived => {
                    Ok(Loop::Continue((latest_stream_event, Instant::now())))
                }
                StatusPing::EnsureAliveConnectionSlack => {
                    println!("EnsureAliveConnectionSlack received");
                    if latest_slack_event.elapsed().as_secs() > 720 {
                        slack::rtm::connect_to_slack(
                            conf::SLACK_BOT_TOKEN,
                            conf::SLACK_BOT_USER_ID,
                            conf::SLACK_CHANNEL,
                            main_tx.clone(),
                            status_tx.clone(),
                        );
                        println!("Slack connection restarted.");
                        Ok(Loop::Continue((latest_stream_event, Instant::now())))
                    } else {
                        Ok(Loop::Continue((latest_stream_event, latest_slack_event)))
                    }
                }
            }
        },
    ));
}

pub fn periodically_ensure_alive_connection(status_tx: Sender<StatusPing>) {
    println!("in periodically_...");
    tokio::spawn(loop_fn((), move |_| {
        println!("New iteration of perodically_ensure_alive_connection");
        let status_tx2 = status_tx.clone();
        Delay::new(Instant::now() + Duration::from_secs(15))
            .and_then(move |_| {
                println!("StatusPing::* sending");
                status_tx2
                    .send(StatusPing::EnsureAliveConnectionLichess)
                    .unwrap();
                status_tx2
                    .send(StatusPing::EnsureAliveConnectionSlack)
                    .unwrap();
                Ok(Loop::Continue(()))
            })
            .map_err(|e| println!("Err in periodically_...: {}", e))
    }));
}
