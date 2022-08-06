use chrono::{prelude::*, Duration};
use event::Event;
use futures::future;
use hyper::header::HeaderValue;
use hyper::rt::Future;
use hyper::{Body, Client, Method, Request};
use hyper_rustls::HttpsConnector;
use lua;
use rand::{thread_rng, Rng};
use signup::rules::*;
use std::collections::{HashMap, VecDeque};
use std::ops::Add;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time;
use tokio;
use zulip;

use crate::event::User;

pub fn handle_events(
    rx: Receiver<Event>,
    token: &'static str,
    rules_path: &'static str,
    zulip_bot_id: &'static str,
    zulip_bot_token: &'static str,
    zulip_main_stream: &'static str,
    zulip_main_topic: &'static str,
    zulip_notify_stream: &'static str,
    zulip_notify_topic: &'static str,
    zulip_url: &'static str,
) {
    let mut rule_manager =
        SignupRulesManager::new(rules_path.to_string()).expect("could not load rules");

    println!("Currently {} rules.", rule_manager.rules.len());

    let mut latest_event_utc: DateTime<Utc> = Utc::now();

    let lua_state = lua::new_lua();

    let mut recently_notified: Vec<String> = vec![];
    let mut recently_checked: VecDeque<String> = VecDeque::new();
    let mut recently_checked_info: HashMap<String, VecDeque<User>> = HashMap::new();

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

                let user_id = user.username.0.to_lowercase();
                recently_checked.push_back(user_id.clone());

                if !recently_checked_info.contains_key(&user_id) {
                    recently_checked_info.insert(user_id.clone(), VecDeque::new());
                }
                recently_checked_info
                    .get_mut(&user_id)
                    .unwrap()
                    .push_back(user.clone());

                if recently_checked.len() > 10000 {
                    let popped = recently_checked.pop_front().unwrap();
                    recently_checked_info
                        .get_mut(&popped)
                        .map(|v| v.pop_front());
                    if recently_checked_info
                        .get(&popped)
                        .eq(&Some(&VecDeque::new()))
                    {
                        recently_checked_info.remove(&popped);
                    }
                }

                let delay_ms_if_needed = thread_rng().gen_range(30..100) * 1000;

                let mut matched_rules: Vec<String> = vec![];

                for rule in &rule_manager.rules {
                    let take_action = if !rule.enabled || rule.has_expired() {
                        Ok(false)
                    } else if rule.susp_ip && !user.susp_ip {
                        Ok(false)
                    } else {
                        rule.criterion.take_action(&user, &lua_state)
                    };

                    if hypothetical && take_action.clone().unwrap_or(false) {
                        zulip::web::post_message(
                            format!(
                                "Rule {} would take these actions: {:?}",
                                &rule.name, &rule.actions
                            ),
                            zulip_bot_id,
                            zulip_bot_token,
                            zulip_main_stream,
                            zulip_main_topic,
                            zulip_url,
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

                                        let https = HttpsConnector::new(1);
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
                                            zulip::web::post_message(
                                                format!(
                                                    "Rule {} match: https://lichess.org/@/{}",
                                                    &rule.name, &user.username.0
                                                ),
                                                zulip_bot_id,
                                                zulip_bot_token,
                                                zulip_notify_stream,
                                                zulip_notify_topic,
                                                zulip_url,
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
                                zulip::web::post_message(
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
                                    zulip_bot_id,
                                    zulip_bot_token,
                                    zulip_main_stream,
                                    zulip_main_topic,
                                    zulip_url,
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
                            zulip::web::post_message(
                                err_msg,
                                zulip_bot_id,
                                zulip_bot_token,
                                zulip_main_stream,
                                zulip_main_topic,
                                zulip_url,
                            );
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
                    zulip::web::post_message(
                        format!("Error on adding rule: {}", err),
                        zulip_bot_id,
                        zulip_bot_token,
                        zulip_main_stream,
                        zulip_main_topic,
                        zulip_url,
                    );
                }
                Ok(_) => {
                    zulip::web::post_message(
                        "Rule added!".to_owned(),
                        zulip_bot_id,
                        zulip_bot_token,
                        zulip_main_stream,
                        zulip_main_topic,
                        zulip_url,
                    );
                }
            },
            Event::InternalShowRule(name) => {
                let zulip_message = match rule_manager.find_rule(name) {
                    None => "No such rule found.".to_owned(),
                    Some(rule) => format!(
                        "Criterion: {}.\nActions: {:?}{}{}",
                        rule.criterion.friendly(),
                        rule.actions,
                        if rule.no_delay { ". No delay" } else { "" },
                        if let Some(expiry) = rule.expiry {
                            format!(". Expires: {}", expiry)
                        } else {
                            "".to_owned()
                        },
                    ),
                };
                zulip::web::post_message(
                    zulip_message,
                    zulip_bot_id,
                    zulip_bot_token,
                    zulip_main_stream,
                    zulip_main_topic,
                    zulip_url,
                );
            }
            Event::InternalRemoveRule(name) => {
                let zulip_message = match rule_manager.remove_rule(name) {
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
                zulip::web::post_message(
                    zulip_message,
                    zulip_bot_id,
                    zulip_bot_token,
                    zulip_main_stream,
                    zulip_main_topic,
                    zulip_url,
                );
            }
            Event::InternalDisableRules(pattern) => {
                let zulip_message = match rule_manager.disable_rules(pattern) {
                    Ok(count) => format!("{} rules disabled.", count),
                    Err(err) => format!("Error on disabling rules: {}", err),
                };
                zulip::web::post_message(
                    zulip_message,
                    zulip_bot_id,
                    zulip_bot_token,
                    zulip_main_stream,
                    zulip_main_topic,
                    zulip_url,
                );
            }
            Event::InternalEnableRules(pattern) => {
                let zulip_message = match rule_manager.enable_rules(pattern) {
                    Ok(count) => format!("{} rules enabled.", count),
                    Err(err) => format!("Error on enabling rules: {}", err),
                };
                zulip::web::post_message(
                    zulip_message,
                    zulip_bot_id,
                    zulip_bot_token,
                    zulip_main_stream,
                    zulip_main_topic,
                    zulip_url,
                );
            }
            Event::InternalListRules => zulip::web::post_message(
                format!("Current rules: {}", rule_manager.list_names().join(", ")),
                zulip_bot_id,
                zulip_bot_token,
                zulip_main_stream,
                zulip_main_topic,
                zulip_url,
            ),
            Event::InternalStreamEventReceived => latest_event_utc = Utc::now(),
            Event::InternalZulipStatusCommand => zulip::web::post_message(
                format!(
                    "I am alive! Latest event: (UTC) {}",
                    latest_event_utc.format("%d/%m/%Y %T")
                ),
                zulip_bot_id,
                zulip_bot_token,
                zulip_main_stream,
                zulip_main_topic,
                zulip_url,
            ),
            Event::InternalIsRecentlyChecked(username) => zulip::web::post_message(
                if recently_checked.contains(&username.to_lowercase()) {
                    let empty_vec = VecDeque::new();
                    let infos = recently_checked_info
                        .get(&username.to_lowercase())
                        .unwrap_or(&empty_vec);
                    let info_string = infos
                        .into_iter()
                        .map(|i| String::from("`") + &serde_json::to_string(i).unwrap() + "`")
                        .collect::<Vec<String>>()
                        .join("\n");
                    format!("Yes, that user has been seen in the latest 10K sign-ins. Seen {} times:\n{}", infos.len(), info_string)
                } else {
                    "No, that user has not been seen in the latest 10K sign-ins.".to_string()
                },
                zulip_bot_id,
                zulip_bot_token,
                zulip_main_stream,
                zulip_main_topic,
                zulip_url,
            ),
            Event::InternalCheckRulesExpiry => {
                let mut rules_to_remove = vec![];

                for mut rule in &mut rule_manager.rules {
                    if let Some(expiry) = rule.expiry {
                        if expiry < Utc::now().add(Duration::days(1)) && rule.exp_notification == 0
                        {
                            zulip::web::post_message(
                                format!(
                                    "Notice: rule `{}` is expiring in less than a day",
                                    rule.name
                                ),
                                zulip_bot_id,
                                zulip_bot_token,
                                zulip_notify_stream,
                                zulip_notify_topic,
                                zulip_url,
                            );
                            rule.exp_notification = 1;
                        } else if expiry > Utc::now() && rule.exp_notification <= 1 {
                            zulip::web::post_message(
                                format!("Notice: rule `{}` has expired", rule.name),
                                zulip_bot_id,
                                zulip_bot_token,
                                zulip_notify_stream,
                                zulip_notify_topic,
                                zulip_url,
                            );
                            rule.exp_notification = 2;
                        }

                        if Utc::now() > expiry.add(Duration::days(3)) {
                            rules_to_remove.push(rule.name.clone());
                        }
                    }
                }

                if let Err(e) = rule_manager.save() {
                    zulip::web::post_message(
                        format!("Error while saving in InternalCheckRulesExpiry: {:?}", e),
                        zulip_bot_id,
                        zulip_bot_token,
                        zulip_notify_stream,
                        zulip_notify_topic,
                        zulip_url,
                    );
                }

                for rule_to_remove in rules_to_remove {
                    if let Err(e) = rule_manager.remove_rule(rule_to_remove) {
                        zulip::web::post_message(
                            format!("Error while automatically removing expired rule: {:?}", e),
                            zulip_bot_id,
                            zulip_bot_token,
                            zulip_notify_stream,
                            zulip_notify_topic,
                            zulip_url,
                        );
                    }
                }
            }
            Event::InternalRenewRule { rule, new_expiry } => {
                zulip::web::post_message(
                    match rule_manager.renew(rule, new_expiry) {
                        Ok(_) => "Rule renewed!".to_owned(),
                        Err(e) => format!("Error on renewing: {:?}", e),
                    },
                    zulip_bot_id,
                    zulip_bot_token,
                    zulip_main_stream,
                    zulip_main_topic,
                    zulip_url,
                );
            }
        }
    }
}
