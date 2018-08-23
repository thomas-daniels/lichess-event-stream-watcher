use event::{Username, Email, Ip, UserAgent, FingerPrint};
use std::fs::File;

pub struct SignupRulesManager {
    rules: SignupRules,
    rules_path: String,
}

#[derive(Serialize, Deserialize)]
struct SignupRules {
    rules: Vec<Rule>
}

impl SignupRulesManager {
    fn new(rules_path: String) -> Result<Self, Box<std::error::Error>> {
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
    criterion: Criterion,
    action: Action,
}

#[derive(Serialize, Deserialize)]
pub enum Criterion {
    IpMatch(Ip),
    PrintMatch(FingerPrint),
    EmailContains(String),
    UsernameContains(String),
    UseragentLengthLte(usize)
}

impl Criterion {
    fn take_action(&self, username: &Username, email: &Email, ip: &Ip, user_agent: &UserAgent, finger_print: &FingerPrint) -> bool {
        match self {
            Criterion::IpMatch(exact) => exact.eq(ip),
            Criterion::PrintMatch(exact) => exact.eq(finger_print),
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
    EnableChatPanic
}
