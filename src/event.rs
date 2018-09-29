use signup::rules::Rule;

#[derive(Deserialize)]
#[serde(tag = "t")]
pub enum Event {
    #[serde(rename_all = "camelCase", rename = "signup")]
    Signup {
        username: Username,
        email: Email,
        ip: Ip,
        user_agent: UserAgent,
        finger_print: Option<FingerPrint>,
    },
    InternalAddRule {
        rule: Rule,
    },
    InternalShowRule(String),
    InternalRemoveRule(String),
    InternalListRules,
    InternalStreamEventReceived,
    InternalSlackStatusCommand,
}

impl Event {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[derive(Deserialize, PartialEq)]
pub struct Username(pub String);

#[derive(Deserialize, PartialEq)]
pub struct Email(pub String);

#[derive(Serialize, Deserialize, PartialEq)]
pub struct Ip(pub String);

#[derive(Deserialize, PartialEq)]
pub struct UserAgent(pub String);

#[derive(Serialize, Deserialize, PartialEq)]
pub struct FingerPrint(pub String);
