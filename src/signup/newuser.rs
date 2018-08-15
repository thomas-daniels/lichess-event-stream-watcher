use serde_json::Error;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub ip: String,
    pub user_agent: String,
    pub finger_print: Option<String>
}

impl NewUser {
    pub fn from_json(json: &str) -> Result<Self, Error> {
        serde_json::from_str(json)
    }
}