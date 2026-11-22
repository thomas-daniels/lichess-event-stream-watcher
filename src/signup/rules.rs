use crate::event::Event;
use crate::event::{FingerPrint, Ip, User, Username};
use crate::lua;

use chrono::Utc;
use chrono::{serde::ts_milliseconds, serde::ts_milliseconds_option, DateTime};
use futures::{
    future::{loop_fn, Loop},
    Future,
};
use regex::Regex;
use rlua;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    sync::mpsc::Sender,
    time::Instant,
};
use tokio::timer::Delay;

pub struct SignupRulesManager {
    pub rules: Vec<Rule>,
    rules_path: String,
}

impl SignupRulesManager {
    pub fn new(rules_path: String) -> Result<Self, Box<dyn std::error::Error>> {
        let f = File::open(&rules_path)?;
        let r = serde_json::from_reader(f)?;
        Ok(SignupRulesManager {
            rules: r,
            rules_path: rules_path,
        })
    }

    pub fn find_rule(&self, name: String) -> Option<&Rule> {
        self.rules.iter().find(|r| r.name.eq(&name))
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let f = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.rules_path)?;
        serde_json::to_writer(f, &self.rules)?;
        Ok(())
    }

    pub fn add_rule(&mut self, rule: Rule) -> Result<(), Box<dyn std::error::Error>> {
        if self.find_rule(rule.name.clone()).is_some() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Already a rule found with that name.",
            )));
        }
        self.rules.push(rule);
        self.save()
    }

    pub fn remove_rule(&mut self, name: String) -> Result<bool, Box<dyn std::error::Error>> {
        let before = self.rules.len();
        self.rules.retain(|r| !r.name.eq(&name));
        let after = self.rules.len();
        self.save()?;
        Ok(before != after)
    }

    fn enable_disable_rules(
        &mut self,
        pattern: String,
        enabled: bool,
    ) -> Result<i32, Box<dyn std::error::Error>> {
        match Regex::new(&pattern) {
            Ok(re) => {
                let mut counter = 0;
                for rule in &mut self.rules {
                    if re.is_match(&rule.name) {
                        counter += 1;
                        rule.enabled = enabled;
                    }
                }
                self.save()?;
                Ok(counter)
            }
            _ => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Invalid regex.",
            ))),
        }
    }

    pub fn disable_rules(&mut self, pattern: String) -> Result<i32, Box<dyn std::error::Error>> {
        self.enable_disable_rules(pattern, false)
    }

    pub fn enable_rules(&mut self, pattern: String) -> Result<i32, Box<dyn std::error::Error>> {
        self.enable_disable_rules(pattern, true)
    }

    pub fn renew(
        &mut self,
        rule_name: String,
        expiry: DateTime<Utc>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        for rule in &mut self.rules {
            if rule.name == rule_name {
                rule.expiry = Some(expiry);
                break;
            }
        }
        self.save()?;
        Ok(())
    }

    pub fn list_by_tag(&self) -> String {
        let mut with_actions: Vec<String> = vec![];
        let mut tagged: BTreeMap<&str, Vec<String>> = BTreeMap::new();
        let mut others: Vec<String> = vec![];
        for r in &self.rules {
            let name = if r.enabled {
                r.name.clone()
            } else {
                format!("({})", &r.name)
            };
            if r.actions.iter().any(|a| *a != Action::NotifyZulip) {
                with_actions.push(name);
                continue;
            }
            match r.tag() {
                Some(tag) => tagged.entry(tag).or_default().push(name),
                None => others.push(name),
            }
        }

        let mut lines = vec![format!("Current rules ({}):", self.rules.len())];
        if !with_actions.is_empty() {
            lines.push(format!(
                "**Actions** ({}): {}",
                with_actions.len(),
                with_actions.join(", ")
            ));
        }
        for (tag, names) in &tagged {
            lines.push(format!("**{}** ({}): {}", tag, names.len(), names.join(", ")));
        }
        if !others.is_empty() {
            lines.push(format!("**Others** ({}): {}", others.len(), others.join(", ")));
        }
        lines.join("\n")
    }

    pub fn caught(
        &mut self,
        name: String,
        user: &Username,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let index = self
            .rules
            .iter()
            .position(|r| r.name.eq(&name))
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Index could not be found.",
            ))?;
        {
            let rule = self.rules.get_mut(index).ok_or(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to find rule for index.",
            ))?;

            if rule.most_recent_caught.contains(&user.0) {
                return Ok(());
            }

            rule.match_count += 1;
            rule.latest_match_date = Some(Utc::now());
            let mrc = &mut rule.most_recent_caught;
            let Username(user) = user;
            mrc.push(user.to_owned());
            if mrc.len() > 3 {
                mrc.remove(0);
            }
        }
        self.save()
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Rule {
    pub name: String,
    pub criterion: Criterion,
    pub actions: Vec<Action>,
    #[serde(default = "default_match_count")]
    pub match_count: usize,
    #[serde(default = "default_mrc")]
    pub most_recent_caught: Vec<String>,
    #[serde(default = "default_nodelay")]
    pub no_delay: bool,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_ip_susp")]
    pub susp_ip: bool,
    #[serde(with = "ts_milliseconds_option", default = "default_expiry")]
    pub expiry: Option<chrono::DateTime<Utc>>,
    #[serde(default = "default_exp_notification")]
    pub exp_notification: u8,
    #[serde(with = "ts_milliseconds", default = "default_creation_date")]
    pub creation_date: DateTime<Utc>,
    #[serde(with = "ts_milliseconds_option", default = "default_latest_match_date")]
    pub latest_match_date: Option<DateTime<Utc>>,
}

fn default_match_count() -> usize {
    0
}
fn default_mrc() -> Vec<String> {
    vec![]
}

fn default_nodelay() -> bool {
    false
}

fn default_enabled() -> bool {
    true
}

fn default_ip_susp() -> bool {
    false
}

fn default_exp_notification() -> u8 {
    0
}

fn default_creation_date() -> DateTime<Utc> {
    DateTime::<Utc>::MIN_UTC
}

fn default_latest_match_date() -> Option<DateTime<Utc>> {
    None
}

fn default_expiry() -> Option<DateTime<Utc>> {
    None
}

impl Rule {
    pub fn has_expired(&self) -> bool {
        if let Some(expiry) = self.expiry {
            Utc::now() > expiry
        } else {
            false
        }
    }

    pub fn tag(&self) -> Option<&str> {
        match self.name.split_once(':') {
            Some((tag, _)) if !tag.is_empty() => Some(tag),
            _ => None,
        }
    }

    pub fn notify_topic(&self, default_topic: &str, tag_topic_prefix: &str) -> String {
        match self.tag() {
            Some(tag) => format!("{}{}", tag_topic_start(tag_topic_prefix), tag),
            None => default_topic.to_owned(),
        }
    }
}

fn tag_topic_start(tag_topic_prefix: &str) -> String {
    format!("{}: ", tag_topic_prefix)
}

pub fn is_tag_topic(topic: &str, tag_topic_prefix: &str) -> bool {
    topic.starts_with(&tag_topic_start(tag_topic_prefix))
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Criterion {
    IpMatch(Ip),
    PrintMatch(FingerPrint),
    EmailContains(String),
    EmailRegex(#[serde(with = "serde_regex")] Regex),
    UsernameContains(String),
    UsernameRegex(#[serde(with = "serde_regex")] Regex),
    UseragentLengthLte(usize),
    Lua(String),
}

impl Criterion {
    pub fn take_action(&self, user: &User, lua_state: &rlua::Lua) -> Result<bool, rlua::Error> {
        Ok(match self {
            Criterion::IpMatch(exact) => exact.eq(&user.ip),
            Criterion::PrintMatch(exact) => match user.finger_print {
                None => false,
                Some(ref fp) => exact.eq(&fp),
            },
            Criterion::EmailContains(part) => {
                user.email.0.to_uppercase().contains(&part.to_uppercase())
            }
            Criterion::EmailRegex(re) => re.is_match(&user.email.0),
            Criterion::UsernameContains(part) => user
                .username
                .0
                .to_uppercase()
                .contains(&part.to_uppercase()),
            Criterion::UsernameRegex(re) => re.is_match(&user.username.0),
            Criterion::UseragentLengthLte(len) => match user.user_agent {
                None => false,
                Some(ref ua) => ua.0.len() <= *len,
            },
            Criterion::Lua(code) => lua::call_constraints_function(code, user.clone(), lua_state)?,
        })
    }

    pub fn friendly(&self) -> String {
        match self {
            Criterion::IpMatch(exact) => format!("IP equals `{}`", exact.0),
            Criterion::PrintMatch(exact) => format!("Fingerprint hash equals `{}`", exact.0),
            Criterion::EmailContains(s) => format!("Email address contains `{}`", s),
            Criterion::EmailRegex(s) => format!("Email address matches regular expression `{}`", s),
            Criterion::UsernameContains(s) => {
                format!("Username contains (case-insensitive) `{}`", s)
            }
            Criterion::UsernameRegex(s) => format!("Username matches regular expression `{}`", s),
            Criterion::UseragentLengthLte(l) => {
                format!("User agent length is less than or equal to {}", l)
            }
            Criterion::Lua(code) => format!("Lua code `{}` evaluates to true.", code),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
pub enum Action {
    Shadowban,
    EngineMark,
    BoostMark,
    IpBan,
    Close,
    Alt,
    EnableChatPanic,
    NotifyZulip,
}

impl Action {
    pub fn api_endpoint(&self, username: &Username) -> Option<String> {
        match self {
            Action::Shadowban => Some(format!("https://lichess.org/mod/{}/troll/true", username.0)),
            Action::EngineMark => Some(format!(
                "https://lichess.org/mod/{}/engine/true",
                username.0
            )),
            Action::BoostMark => Some(format!(
                "https://lichess.org/mod/{}/booster/true",
                username.0
            )),
            Action::IpBan => Some(format!("https://lichess.org/mod/{}/ban/true", username.0)),
            Action::Close => Some(format!("https://lichess.org/mod/{}/close", username.0)),
            Action::Alt => Some(format!("https://lichess.org/mod/{}/alt/true", username.0)),
            Action::EnableChatPanic => Some(String::from("https://lichess.org/mod/chat-panic")),
            Action::NotifyZulip => None,
        }
    }
}

pub fn expiry_loop(event_tx: Sender<Event>) {
    println!("Expiry loop started.");
    tokio::spawn(loop_fn((), move |_| {
        let event_tx2 = event_tx.clone();
        Delay::new(Instant::now() + std::time::Duration::from_secs(15 * 60))
            .and_then(move |_| {
                event_tx2.send(Event::InternalCheckRulesExpiry).unwrap();
                Ok(Loop::Continue(()))
            })
            .map_err(|e| println!("Err in periodically_...: {}", e))
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule_named(name: &str) -> Rule {
        serde_json::from_str(&format!(
            r#"{{"name": "{}", "criterion": {{"IpMatch": "127.0.0.1"}}, "actions": []}}"#,
            name
        ))
        .unwrap()
    }

    #[test]
    fn tag_is_text_before_first_colon() {
        assert_eq!(rule_named("Ru:some-rule").tag(), Some("Ru"));
        assert_eq!(rule_named("Ru:a:b").tag(), Some("Ru"));
        assert_eq!(rule_named("plain-rule").tag(), None);
        assert_eq!(rule_named(":no-tag").tag(), None);
    }

    #[test]
    fn notify_topic_uses_prefix_for_tagged_rules_only() {
        assert_eq!(
            rule_named("Ru:some-rule").notify_topic("default", "Watcher"),
            "Watcher: Ru"
        );
        assert_eq!(
            rule_named("plain-rule").notify_topic("default", "Watcher"),
            "default"
        );
    }

    #[test]
    fn list_puts_action_rules_first_then_groups_by_tag_with_others_last() {
        let mut disabled = rule_named("Ru:c");
        disabled.enabled = false;
        let mut tagged_ban = rule_named("Ru:ban");
        tagged_ban.actions = vec![Action::Shadowban, Action::NotifyZulip];
        let mut close = rule_named("closer");
        close.actions = vec![Action::Close];
        let mut notify_only = rule_named("Ru:a");
        notify_only.actions = vec![Action::NotifyZulip];
        let manager = SignupRulesManager {
            rules: vec![
                rule_named("rule1"),
                tagged_ban,
                notify_only,
                rule_named("De:x"),
                close,
                rule_named("Ru:b"),
                disabled,
            ],
            rules_path: String::new(),
        };
        assert_eq!(
            manager.list_by_tag(),
            "Current rules (7):\n\
             **Actions** (2): Ru:ban, closer\n\
             **De** (1): De:x\n\
             **Ru** (3): Ru:a, Ru:b, (Ru:c)\n\
             **Others** (1): rule1"
        );
    }
}
