#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum Event {
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
