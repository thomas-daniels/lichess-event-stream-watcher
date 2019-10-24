use signup::rules::Rule;

#[derive(Deserialize, Clone)]
#[serde(tag = "t")]
pub enum Event {
    #[serde(rename_all = "camelCase", rename = "signup")]
    Signup(User),
    InternalHypotheticalSignup(User),
    InternalAddRule {
        rule: Rule,
    },
    InternalShowRule(String),
    InternalRemoveRule(String),
    InternalDisableRules(String),
    InternalEnableRules(String),
    InternalListRules,
    InternalStreamEventReceived,
    InternalSlackStatusCommand,
    InternalIsRecentlyChecked(String),
}

impl Event {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub username: Username,
    pub email: Email,
    pub ip: Ip,
    pub user_agent: UserAgent,
    pub finger_print: Option<FingerPrint>,
    #[serde(default = "default_susp_ip")]
    pub susp_ip: bool,
}

impl User {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

fn default_susp_ip() -> bool {
    false
}

#[derive(Deserialize, PartialEq, Clone)]
pub struct Username(pub String);

#[derive(Deserialize, PartialEq, Clone)]
pub struct Email(pub String);

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct Ip(pub String);

#[derive(Deserialize, PartialEq, Clone)]
pub struct UserAgent(pub String);

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct FingerPrint(pub String);
