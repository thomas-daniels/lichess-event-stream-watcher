use chrono::prelude::*;
use event::Event;
use futures::future;
use hyper::header::HeaderValue;
use hyper::rt::Future;
use hyper::{Body, Client, Method, Request};
use hyper_tls::HttpsConnector;
use lua;
use rand::{thread_rng, Rng};
use signup::rules::*;
use slack;
use std::collections::VecDeque;
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

    let mut latest_event_utc: DateTime<Utc> = Utc::now();

    let lua_state = lua::new_lua();

    let mut recently_notified: Vec<String> = vec![];
    let mut recently_checked: VecDeque<String> = VecDeque::new();

    loop {
        let event = rx.recv().unwrap();
        let event2 = event.clone();

        match event {
            Event::Signup(user) | Event::InternalHypotheticalSignup(user) => {
                let hypothetical = match event2 {
                    Event::Signup(_) => false,
                    Event::InternalHypotheticalSignup(_) => true,
                    _ => panic!("This is impossible."),
                };

                recently_checked.push_back(user.username.0.to_lowercase());
                if recently_checked.len() > 10000 {
                    recently_checked.pop_front();
                }

                let delay_ms_if_needed = thread_rng().gen_range(30, 180) * 1000;

                let mut matched_rules: Vec<String> = vec![];

                for rule in &rule_manager.rules {
                    let take_action = if !rule.enabled {
                        Ok(false)
                    } else if rule.susp_ip && !user.susp_ip {
                        Ok(false)
                    } else {
                        rule.criterion.take_action(&user, &lua_state)
                    };

                    if hypothetical && take_action.clone().unwrap_or(false) {
                        slack::web::post_message(
                            format!(
                                "Rule {} would take these actions: {:?}",
                                &rule.name, &rule.actions
                            ),
                            slack_token,
                            slack_channel,
                        );
                    }
                    let take_real_action = if take_action.is_ok() {
                        Ok(take_action.unwrap() && !hypothetical)
                    } else {
                        take_action
                    };

                    match take_real_action {
                        Ok(true) => {
                            matched_rules.push(rule.name.clone());

                            let bearer = "Bearer ".to_owned() + token;

                            for action in &rule.actions {
                                match action.api_endpoint(&user.username) {
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

                                        let delay = !rule.no_delay
                                            && (action.eq(&Action::EngineMark)
                                                || action.eq(&Action::BoostMark)
                                                || action.eq(&Action::IpBan)
                                                || action.eq(&Action::Close));

                                        let delay_additional =
                                            if !rule.no_delay && action.eq(&Action::Close) {
                                                1500
                                            } else {
                                                0
                                            };

                                        tokio::spawn(future::lazy(move || {
                                            if delay {
                                                thread::sleep(time::Duration::from_millis(
                                                    delay_ms_if_needed + delay_additional,
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
                                        if action.eq(&Action::NotifySlack)
                                            && !recently_notified.contains(&user.username.0)
                                        {
                                            slack::web::post_message(
                                                format!(
                                                    "Rule {} match: https://lichess.org/@/{}",
                                                    &rule.name, &user.username.0
                                                ),
                                                slack_token,
                                                slack_notify_channel,
                                            );

                                            recently_notified.insert(0, user.username.0.clone());
                                            if recently_notified.len() > 5 {
                                                recently_notified.pop();
                                            }
                                        }
                                    }
                                }
                            }

                            if rule.actions.len() > 1
                                || !rule.actions.get(0).eq(&Some(&Action::NotifySlack))
                            {
                                slack::web::post_message(
                                    format!(
                                        "Rule {} match: \
                                         {} on <https://lichess.org/@/{}?mod|{}>. \
                                         {} previous matches. \
                                         Recent matches: {}",
                                        &rule.name,
                                        &rule.criterion.friendly(),
                                        &user.username.0,
                                        &user.username.0,
                                        &rule.match_count,
                                        if rule.most_recent_caught.len() == 0 {
                                            "None".to_string()
                                        } else {
                                            rule.most_recent_caught
                                                .iter()
                                                .map(|u| {
                                                    format!(
                                                        "<https://lichess.org/@/{}?mod|{}>",
                                                        &u, &u
                                                    )
                                                })
                                                .collect::<Vec<String>>()
                                                .join(", ")
                                        }
                                    ),
                                    slack_token,
                                    slack_channel,
                                );
                            }
                        }
                        Ok(false) => {}
                        Err(err) => {
                            let err_msg = format!(
                                "Error on `{}` for user `{}` (probably in Lua snippet): `{}`",
                                &rule.name, &user.username.0, err
                            );
                            println!("{}", err_msg.clone());
                            slack::web::post_message(err_msg, slack_token, slack_channel);
                        }
                    }
                }

                if !hypothetical {
                    for name in matched_rules {
                        match rule_manager.caught(name, &user.username) {
                            Ok(_) => {}
                            Err(e) => println!("Error in .caught: {}", e),
                        };
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
                        "Criterion: {}.\nActions: {:?}{}",
                        rule.criterion.friendly(),
                        rule.actions,
                        if rule.no_delay { ". No delay." } else { "" }
                    ),
                };
                slack::web::post_message(slack_message, slack_token, slack_channel);
            }
            Event::InternalRemoveRule(name) => {
                let slack_message = match rule_manager.remove_rule(name) {
                    Ok(removed) => {
                        if removed {
                            "Rule removed!".to_owned()
                        } else {
                            "No such rule found.".to_owned()
                        }
                    }
                    Err(err) => {
                        println!("Error on .remove_rule: {}", err);
                        format!("Error on removing rule: {}", err)
                    }
                };
                slack::web::post_message(slack_message, slack_token, slack_channel);
            }
            Event::InternalDisableRules(pattern) => {
                let slack_message = match rule_manager.disable_rules(pattern) {
                    Ok(count) => format!("{} rules disabled.", count),
                    Err(err) => format!("Error on disabling rules: {}", err),
                };
                slack::web::post_message(slack_message, slack_token, slack_channel);
            }
            Event::InternalEnableRules(pattern) => {
                let slack_message = match rule_manager.enable_rules(pattern) {
                    Ok(count) => format!("{} rules enabled.", count),
                    Err(err) => format!("Error on enabling rules: {}", err),
                };
                slack::web::post_message(slack_message, slack_token, slack_channel);
            }
            Event::InternalListRules => slack::web::post_message(
                format!("Current rules: {}", rule_manager.list_names().join(", ")),
                slack_token,
                slack_channel,
            ),
            Event::InternalStreamEventReceived => latest_event_utc = Utc::now(),
            Event::InternalSlackStatusCommand => slack::web::post_message(
                format!(
                    "I am alive! Latest event: (UTC) {}",
                    latest_event_utc.format("%d/%m/%Y %T")
                ),
                slack_token,
                slack_channel,
            ),
            Event::InternalIsRecentlyChecked(username) => slack::web::post_message(
                if recently_checked.contains(&username.to_lowercase()) {
                    "Yes, that user has been seen in the latest 10K sign-ins.".to_string()
                } else {
                    "No, that user has not been seen in the latest 10K sign-ins.".to_string()
                },
                slack_token,
                slack_channel,
            ),
        }
    }
}
