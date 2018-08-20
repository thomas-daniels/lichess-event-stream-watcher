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
}

impl Event {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

macro_rules! quick_stringify {
    ($t:path) => {
        impl<'a> std::ops::Not for &'a $t {
            type Output = &'a String;

            fn not(self) -> &'a String {
                let $t(s) = self;

                s
            }
        }
    }
}

#[derive(Deserialize, PartialEq)]
pub struct Username(pub String);

quick_stringify!(Username);

#[derive(Deserialize, PartialEq)]
pub struct Email(pub String);

quick_stringify!(Email);

#[derive(Serialize, Deserialize, PartialEq)]
pub struct Ip(pub String);

quick_stringify!(Ip);

#[derive(Deserialize, PartialEq)]
pub struct UserAgent(pub String);

quick_stringify!(UserAgent);

#[derive(Serialize, Deserialize, PartialEq)]
pub struct FingerPrint(pub String);

quick_stringify!(FingerPrint);
