#[derive(Deserialize)]
#[serde(tag="t")]
pub enum Event {
    #[serde(rename_all="camelCase", rename="signup")]
    Signup {
        username: Username,
        email: Email,
        ip: Ip,
        user_agent: UserAgent,
        finger_print: Option<FingerPrint>
    }
}

impl Event {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[derive(Deserialize)]
pub struct Username(pub String);

#[derive(Deserialize)]
pub struct Email(pub String);

#[derive(Deserialize)]
pub struct Ip(pub String);

#[derive(Deserialize)]
pub struct UserAgent(pub String);

#[derive(Deserialize)]
pub struct FingerPrint(pub String);