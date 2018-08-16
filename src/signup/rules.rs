use event::Event;

trait SignupRule {
    fn let_in(&self, ip: &String) -> bool;
}

pub struct IpBlacklist {
    blacklisted: Vec<String>,
}

impl IpBlacklist {
    fn new(blacklist: Vec<String>) -> Self {
        IpBlacklist {
            blacklisted: blacklist,
        }
    }
}

impl SignupRule for IpBlacklist {
    fn let_in(&self, ip: &String) -> bool {
        !self.blacklisted.contains(ip)
    }
}
