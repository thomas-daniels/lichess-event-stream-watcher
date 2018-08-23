use event::{Email, FingerPrint, Ip, UserAgent, Username};
use std::fs::File;

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
}

#[derive(Serialize, Deserialize)]
pub struct Rule {
    pub criterion: Criterion,
    pub action: Action,
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
            Criterion::EmailContains(part) => email.0.contains(part),
            Criterion::UsernameContains(part) => username.0.contains(part),
            Criterion::UseragentLengthLte(len) => user_agent.0.len() <= *len,
        }
    }
}

#[derive(Serialize, Deserialize)]
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
    pub fn api_endpoint(&self, username: &Username) -> String {
        match self {
            Action::Shadowban => format!("https://lichess.org/mod/{}/troll/true", username.0),
            Action::EngineMark => format!("https://lichess.org/mod/{}/engine/true", username.0),
            Action::BoostMark => format!("https://lichess.org/mod/{}/booster/true", username.0),
            Action::IpBan => format!("https://lichess.org/mod/{}/ban/true", username.0),
            Action::Close => format!("https://lichess.org/mod/{}/close", username.0),
            Action::EnableChatPanic => String::from("https://lichess.org/mod/chat-panic"),
            Action::NotifySlack => format!("https://lichess.org/mod/{}/notify-slack", username.0),
        }
    }
}
