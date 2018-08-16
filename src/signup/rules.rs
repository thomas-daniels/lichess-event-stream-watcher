use event;
use std::fs::File;

pub struct SignupRulesManager {
    rules: SignupRules,
    rules_path: String,
}

#[derive(Serialize, Deserialize)]
struct SignupRules {
    ip_blacklist: Vec<event::Ip>,
    finger_blacklist: Vec<event::FingerPrint>,
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
