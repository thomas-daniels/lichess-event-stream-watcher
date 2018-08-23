#[derive(Serialize, Deserialize)]
pub enum Action {
    Shadowban,
    EngineMark,
    BoostMark,
    IpBan,
    Close,
    EnableChatPanic
}