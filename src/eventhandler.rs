use event::Event;
use futures::future;
use hyper::header::HeaderValue;
use hyper::rt::Future;
use hyper::{Body, Client, Method, Request};
use hyper_tls::HttpsConnector;
use rand::{thread_rng, Rng};
use signup::rules::*;
use slack;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time;
use tokio;

pub fn handle_events(
    rx: Receiver<Event>,
    token: &'static str,
    rules_path: &'static str,
    slack_token: &'static str,
    slack_channel: &'static str,
    slack_notify_channel: &'static str,
) {
    let mut rule_manager =
        SignupRulesManager::new(rules_path.to_string()).expect("could not load rules");
    println!("Currently {} rules.", rule_manager.rules.len());

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
                for rule in &rule_manager.rules {
                    if rule.criterion.take_action(
                        &username,
                        &email,
                        &ip,
                        &user_agent,
                        &finger_print,
                    ) {
                        let bearer = "Bearer ".to_owned() + token;

                        let delay_ms_if_needed = thread_rng().gen_range(30, 180) * 1000;

                        for action in &rule.actions {
                            match action.api_endpoint(&username) {
                                Some(endpoint) => {
                                    let mut action_req = Request::new(Body::from(""));
                                    *action_req.uri_mut() = endpoint.parse().unwrap();
                                    *action_req.method_mut() = Method::POST;
                                    action_req.headers_mut().insert(
                                        hyper::header::AUTHORIZATION,
                                        HeaderValue::from_str(&bearer).unwrap(),
                                    );

                                    let https = HttpsConnector::new(1).unwrap();
                                    let client = Client::builder().build::<_, Body>(https);

                                    let delay = action.eq(&Action::EngineMark)
                                        || action.eq(&Action::BoostMark)
                                        || action.eq(&Action::IpBan)
                                        || action.eq(&Action::Close);

                                    tokio::spawn(future::lazy(move || {
                                        if delay {
                                            thread::sleep(time::Duration::from_millis(
                                                delay_ms_if_needed,
                                            ));
                                        }

                                        client
                                            .request(action_req)
                                            .map(|res| println!("Action: {}.", res.status()))
                                            .map_err(|err| {
                                                println!("Error on mod action: {}", err);
                                            })
                                    }));
                                }
                                None => {
                                    if action.eq(&Action::NotifySlack) {
                                        slack::web::post_message(
                                            format!(
                                                "Rule {} match: https://lichess.org/@/{}",
                                                &rule.name, &username.0
                                            ),
                                            slack_token,
                                            slack_notify_channel,
                                        );
                                    }
                                }
                            }
                        }

                        if rule.actions.len() > 1
                            || !rule.actions.get(0).eq(&Some(&Action::NotifySlack))
                        {
                            slack::web::post_message(
                                format!(
                                    "Rule {} match: automatic actions \
                                     have been taken (or will be taken after a short delay) \
                                     on https://lichess.org/@/{}",
                                    &rule.name, &username.0
                                ),
                                slack_token,
                                slack_channel,
                            );
                        }
                    }
                }
            }
            Event::InternalAddRule { rule } => match rule_manager.add_rule(rule) {
                Err(err) => {
                    println!("Error on .add_rule: {}", err);
                    slack::web::post_message(
                        format!("Error on adding rule: {}", err),
                        slack_token,
                        slack_channel,
                    );
                }
                Ok(_) => {
                    slack::web::post_message("Rule added!".to_owned(), slack_token, slack_channel);
                }
            },
            Event::InternalShowRule(name) => {
                let slack_message = match rule_manager.find_rule(name) {
                    None => "No such rule found.".to_owned(),
                    Some(rule) => format!(
                        "Criterion: {}.\nActions: {:?}",
                        rule.criterion.friendly(),
                        rule.actions
                    ),
                };
                slack::web::post_message(slack_message, slack_token, slack_channel);
            }
            Event::InternalRemoveRule(name) => {
                let slack_message = match rule_manager.remove_rule(name) {
                    Ok(removed) => if removed {
                        "Rule removed!".to_owned()
                    } else {
                        "No such rule found.".to_owned()
                    },
                    Err(err) => {
                        println!("Error on .remove_rule: {}", err);
                        format!("Error on removing rule: {}", err)
                    }
                };
                slack::web::post_message(slack_message, slack_token, slack_channel);
            }
            Event::InternalListRules => slack::web::post_message(
                format!("Current rules: {}", rule_manager.list_names().join(", ")),
                slack_token,
                slack_channel,
            ),
        }
    }
}
