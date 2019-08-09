use event::{FingerPrint, Ip, User, Username};
use lua;
use regex::Regex;
use rlua;
use std::fs::{File, OpenOptions};

pub struct SignupRulesManager {
    pub rules: Vec<Rule>,
    rules_path: String,
}

impl SignupRulesManager {
    pub fn new(rules_path: String) -> Result<Self, Box<std::error::Error>> {
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

    fn save(&self) -> Result<(), Box<std::error::Error>> {
        let f = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&self.rules_path)?;
        serde_json::to_writer(f, &self.rules)?;
        Ok(())
    }

    pub fn add_rule(&mut self, rule: Rule) -> Result<(), Box<std::error::Error>> {
        if self.find_rule(rule.name.clone()).is_some() {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Already a rule found with that name.",
            )));
        }
        self.rules.push(rule);
        self.save()
    }

    pub fn remove_rule(&mut self, name: String) -> Result<bool, Box<std::error::Error>> {
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
    ) -> Result<i32, Box<std::error::Error>> {
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

    pub fn disable_rules(&mut self, pattern: String) -> Result<i32, Box<std::error::Error>> {
        self.enable_disable_rules(pattern, false)
    }

    pub fn enable_rules(&mut self, pattern: String) -> Result<i32, Box<std::error::Error>> {
        self.enable_disable_rules(pattern, true)
    }

    pub fn list_names(&self) -> Vec<String> {
        self.rules
            .iter()
            .map(|r| {
                if r.enabled {
                    r.name.clone()
                } else {
                    format!("({})", &r.name)
                }
            })
            .collect()
    }

    pub fn caught(&mut self, name: String, user: &Username) -> Result<(), Box<std::error::Error>> {
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
            Criterion::EmailRegex(re) => re.is_match(&user.email.0.to_lowercase()),
            Criterion::UsernameContains(part) => user
                .username
                .0
                .to_uppercase()
                .contains(&part.to_uppercase()),
            Criterion::UsernameRegex(re) => re.is_match(&user.username.0.to_lowercase()),
            Criterion::UseragentLengthLte(len) => user.user_agent.0.len() <= *len,
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
    EnableChatPanic,
    NotifySlack,
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
            Action::EnableChatPanic => Some(String::from("https://lichess.org/mod/chat-panic")),
            Action::NotifySlack => None,
        }
    }
}
