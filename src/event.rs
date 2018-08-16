#[derive(Deserialize)]
#[serde(tag="t")]
pub enum Event {
    #[serde(rename_all="camelCase", rename="signup")]
    Signup {
        username: String,
        email: String,
        ip: String,
        user_agent: String,
        finger_print: Option<String>
    }
}

impl Event {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}