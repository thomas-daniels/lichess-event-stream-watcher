use signup::newuser::NewUser;

trait SignupRule {
    fn let_in(&self, user: &NewUser) -> bool;
}

pub struct IpBlacklist {
    blacklisted: Vec<String>
}

impl IpBlacklist {
    fn new(blacklist: Vec<String>) -> Self {
        IpBlacklist {
            blacklisted: blacklist
        }
    }
}

impl SignupRule for IpBlacklist {
    fn let_in(&self, user: &NewUser) -> bool {
        !self.blacklisted.contains(&user.ip)
    }
}