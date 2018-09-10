use event::{Email, FingerPrint, Ip, UserAgent, Username};
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

    pub fn list_names(&self) -> Vec<String> {
        self.rules.iter().map(|r| r.name.clone()).collect()
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

#[derive(Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub criterion: Criterion,
    pub actions: Vec<Action>,
    #[serde(default = "default_match_count")]
    pub match_count: usize,
    #[serde(default = "default_mrc")]
    pub most_recent_caught: Vec<String>,
}

fn default_match_count() -> usize {
    0
}
fn default_mrc() -> Vec<String> {
    vec![]
}

#[derive(Serialize, Deserialize)]
pub enum Criterion {
    IpMatch(Ip),
    PrintMatch(FingerPrint),
    EmailContains(String),
    UsernameContains(String),
    UseragentLengthLte(usize),
}

impl Criterion {
    pub fn take_action(
        &self,
        username: &Username,
        email: &Email,
        ip: &Ip,
        user_agent: &UserAgent,
        finger_print: &Option<FingerPrint>,
    ) -> bool {
        match self {
            Criterion::IpMatch(exact) => exact.eq(ip),
            Criterion::PrintMatch(exact) => match finger_print {
                None => false,
                Some(fp) => exact.eq(fp),
            },
            Criterion::EmailContains(part) => email.0.to_uppercase().contains(&part.to_uppercase()),
            Criterion::UsernameContains(part) => {
                username.0.to_uppercase().contains(&part.to_uppercase())
            }
            Criterion::UseragentLengthLte(len) => user_agent.0.len() <= *len,
        }
    }

    pub fn friendly(&self) -> String {
        match self {
            Criterion::IpMatch(exact) => format!("IP equals `{}`", exact.0),
            Criterion::PrintMatch(exact) => format!("Fingerprint hash equals `{}`", exact.0),
            Criterion::EmailContains(s) => format!("Email address contains `{}`", s),
            Criterion::UsernameContains(s) => {
                format!("Username contains (case-insensitive) `{}`", s)
            }
            Criterion::UseragentLengthLte(l) => {
                format!("User agent length is less than or equal to {}", l)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
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
