use chrono::{DateTime, Utc};
use maxminddb::geoip2;
use signup::rules::Rule;

#[derive(Deserialize, Clone)]
#[serde(tag = "t")]
pub enum Event {
    #[serde(rename_all = "camelCase", rename = "signup")]
    Signup(User),
    InternalHypotheticalSignup(User),
    InternalAddRule {
        rule: Rule,
    },
    InternalShowRule(String),
    InternalRemoveRule(String),
    InternalDisableRules(String),
    InternalEnableRules(String),
    InternalListRules,
    InternalStreamEventReceived,
    InternalZulipStatusCommand,
    InternalIsRecentlyChecked(String),
    InternalCheckRulesExpiry,
    InternalRenewRule {
        rule: String,
        new_expiry: DateTime<Utc>,
    },
}

impl Event {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub username: Username,
    pub email: Email,
    pub ip: Ip,
    pub user_agent: Option<UserAgent>,
    pub finger_print: Option<FingerPrint>,
    #[serde(default = "default_susp_ip")]
    pub susp_ip: bool,
    pub geoip: Option<GeoipInfo>,
}

impl User {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

fn default_susp_ip() -> bool {
    false
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct GeoipInfo {
    pub country: Option<String>,
    pub city: Option<String>,
    pub subdivisions: Option<Vec<String>>,
}

impl GeoipInfo {
    pub fn from_maxminddb_city(city: geoip2::City) -> GeoipInfo {
        GeoipInfo {
            country: city
                .country
                .and_then(|x| x.names)
                .map(|y| y["en"].to_owned()),
            city: city.city.and_then(|x| x.names).map(|y| y["en"].to_owned()),
            subdivisions: city.subdivisions.map(|z| {
                z.iter()
                    .flat_map(|x| (&x.names).as_ref().map(|y| y["en"].to_owned()))
                    .collect()
            }),
        }
    }
}

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Username(pub String);

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Email(pub String);

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Ip(pub String);

#[derive(Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct UserAgent(pub String);

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct FingerPrint(pub String);
