use crate::signup::rules::Rule;

use chrono::{DateTime, Utc};
use lazy_static::lazy_static;
use maxminddb::geoip2;
use regex::Regex;
use serde::{Deserialize, Serialize};
use uaparser::{Parser, UserAgentParser};

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
    pub device: Option<DeviceInfo>,
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

lazy_static! {
    static ref MOB_UA_RE: Regex = Regex::new(
        r"(?i)lichess mobile/(\S+)(?: \(\d*\))? as:(\S+) sri:(\S+) os:(Android|iOS)/(\S+) dev:(.*)"
    )
    .unwrap();
    static ref MOB_UA_TRIM_RE: Regex = Regex::new(r"LM/(\S+) (Android|iOS)/(\S+) (.*)").unwrap();
}

#[derive(Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct DeviceInfo {
    pub device: String,
    pub os: String,
    pub client: String,
}

impl DeviceInfo {
    pub fn lichess_bot(ua: &str) -> Option<DeviceInfo> {
        if ua.starts_with("lichess-bot/") {
            Some(DeviceInfo {
                client: "lichess-bot ".to_string() + &ua[12..].split(" ").next().unwrap_or(""),
                os: String::from("Other"),
                device: String::from("Computer"),
            })
        } else {
            None
        }
    }

    pub fn lichess_mob(ua: &str) -> Option<DeviceInfo> {
        let maybe_caps = MOB_UA_RE.captures(ua);
        maybe_caps.map(|caps| {
            let version = caps.get(0).map(|m| m.as_str()).unwrap_or("?");
            let os_name = caps.get(3).map(|m| m.as_str()).unwrap_or("?");
            let os_version = caps.get(4).map(|m| m.as_str()).unwrap_or("?");
            let device = caps.get(5).map(|m| m.as_str()).unwrap_or("?");

            DeviceInfo {
                device: device.to_string(),
                os: String::from(os_name) + " " + os_version,
                client: String::from("Lichess Mobile ") + version,
            }
        })
    }

    pub fn lichess_mob_trim(ua: &str) -> Option<DeviceInfo> {
        let maybe_caps = MOB_UA_TRIM_RE.captures(ua);
        maybe_caps.map(|caps| {
            let version = caps.get(0).map(|m| m.as_str()).unwrap_or("?");
            let os_name = caps.get(1).map(|m| m.as_str()).unwrap_or("?");
            let os_version = caps.get(2).map(|m| m.as_str()).unwrap_or("?");
            let device = caps.get(3).map(|m| m.as_str()).unwrap_or("?");

            DeviceInfo {
                device: device.to_string(),
                os: String::from(os_name) + " " + os_version,
                client: String::from("Lichess Mobile ") + version,
            }
        })
    }

    pub fn from_uap_client(c: uaparser::Client) -> DeviceInfo {
        let device = if c.device.family == "Other" {
            "Computer"
        } else {
            &c.device.family
        };
        let os = match c.os.major {
            Some(major) => c.os.family + " " + major,
            None => c.os.family,
        };
        let client = match c.user_agent.major {
            Some(major) => c.user_agent.family + " " + major,
            None => c.user_agent.family,
        };
        DeviceInfo {
            device: device.to_string(),
            os: os.to_string(),
            client: client.to_string(),
        }
    }

    pub fn parse_user_agent(ua: &str, parser: &UserAgentParser) -> DeviceInfo {
        DeviceInfo::lichess_bot(ua)
            .or_else(|| DeviceInfo::lichess_mob(ua))
            .or_else(|| DeviceInfo::lichess_mob_trim(ua))
            .unwrap_or_else(|| {
                let parsed = parser.parse(ua);
                DeviceInfo::from_uap_client(parsed)
            })
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
