use event::{Event, Ip, Username};
use futures::future;
use hyper::header::HeaderValue;
use hyper::rt::{Future, Stream};
use hyper::{Body, Client, Request};
use hyper_tls::HttpsConnector;
use signup::rules::*;

pub fn watch_event_stream(token: &'static str, rules_path: &'static str) {
    tokio::run(future::lazy(move || {
        let rule_manager = SignupRulesManager::new(rules_path.to_string()).expect("could not load rules");
        println!("Currently {} rules.", rule_manager.rules.len());

        let https = HttpsConnector::new(4).unwrap();
        let client = Client::builder().build::<_, Body>(https);

        let mut req = Request::new(Body::from(""));
        *req.uri_mut() = "https://lichess.org/api/stream/mod".parse().unwrap();

        let bearer = "Bearer ".to_owned() + token;

        req.headers_mut().insert(
            hyper::header::AUTHORIZATION,
            HeaderValue::from_str(&bearer).unwrap(),
        );

        client
            .request(req)
            .and_then(move |res| {
                res.into_body().for_each(move |chunk| {
                    let string_chunk = &String::from_utf8(chunk.into_bytes().to_vec())
                        .unwrap_or("invalid chunk bytes".to_string());
                    match Event::from_json(string_chunk) {
                        Ok(event) => match event {
                            Event::Signup { username, email, ip, user_agent, finger_print } => {
                                println!("{}", username.0);
                                for rule in &rule_manager.rules {
                                    if (rule.criterion.take_action(&username, &email, &ip, &user_agent, &finger_print)) {
                                        let mut action_req = Request::new(Body::from(""));
                                        *action_req.uri_mut() = rule.action.api_endpoint(&username).parse().unwrap();
                                        action_req.headers_mut().insert(
                                            hyper::header::AUTHORIZATION,
                                            HeaderValue::from_str(&bearer).unwrap(),
                                        );
                                        // TODO: perform action_req
                                    }
                                }
                            }
                        },
                        _ => {
                            println!("deserialize error for {}", string_chunk);
                        }
                    };
                    Ok(())
                })
            }).map_err(|err| {
                println!("Error on get: {}", err);
            })
    }));
}
