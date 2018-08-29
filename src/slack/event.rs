#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum RtmRecv {
    #[serde(rename = "message")]
    Message {
        user: String,
        text: String,
        client_msg_id: String,
        team: String,
        channel: String,
        event_ts: String,
        ts: String,
    },
}

#[derive(Serialize)]
pub struct RtmSend {
    pub id: i32,
    #[serde(rename = "type")]
    pub type_: String,
    pub channel: String,
    pub text: String,
}
