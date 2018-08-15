pub struct NewUser {
    pub username: String,
    pub email: String,
    pub ip: String,
    pub user_agent: String,
    pub fingerprint: Option<String>
}