use event::Event;
use hyper::header::HeaderValue;
use hyper::rt::{Future, Stream};
use hyper::{Body, Client, Request};
use hyper_tls::HttpsConnector;
use signup::rules::*;
use std::sync::mpsc::Receiver;

pub fn handle_events(rx: Receiver<Event>, token: &'static str, rules_path: &'static str) {
    let rule_manager =
        SignupRulesManager::new(rules_path.to_string()).expect("could not load rules");
    println!("Currently {} rules.", rule_manager.rules.len());

    let https = HttpsConnector::new(4).unwrap();
    let client = Client::builder().build::<_, Body>(https);

    let bearer = "Bearer ".to_owned() + token;

    loop {
        let event = rx.recv().unwrap();

        match event {
            Event::Signup {
                username,
                email,
                ip,
                user_agent,
                finger_print,
            } => {
                println!("{}", username.0);
                for rule in &rule_manager.rules {
                    if (rule.criterion.take_action(
                        &username,
                        &email,
                        &ip,
                        &user_agent,
                        &finger_print,
                    )) {
                        let mut action_req = Request::new(Body::from(""));
                        *action_req.uri_mut() =
                            rule.action.api_endpoint(&username).parse().unwrap();
                        action_req.headers_mut().insert(
                            hyper::header::AUTHORIZATION,
                            HeaderValue::from_str(&bearer).unwrap(),
                        );
                        // TODO: perform action_req
                    }
                }
            }
        }
    }
}
