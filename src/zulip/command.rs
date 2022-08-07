use chrono::{Duration, Utc};
use event::{Email, Event, Ip, User};
use regex::Regex;
use serde_json;
use signup::rules::{Action, Criterion, Rule};
use std::error::Error;
use std::sync::mpsc::Sender;

pub fn handle_command(command: String, tx: Sender<Event>) -> Result<Option<String>, ParseError> {
    let cmd = command.clone();
    let parts: Vec<&str> = cmd.split(" ").collect();
    match parts.get(0).ok_or(parse_error(None))? {
        &"status" => handle_status_command(tx.clone()),
        &"signup" => handle_signup_command(command, tx.clone()),
        _ => Err(parse_error(None)),
    }
}

fn handle_status_command(tx: Sender<Event>) -> Result<Option<String>, ParseError> {
    tx.send(Event::InternalZulipStatusCommand).unwrap();
    Ok(None)
}

fn handle_signup_command(command: String, tx: Sender<Event>) -> Result<Option<String>, ParseError> {
    let mut first_split: Vec<&str> = command.split("`").collect();
    let mut code = "";
    if first_split.len() > 2 {
        code = first_split.get(1).ok_or(parse_error(None))?.clone();
        first_split[0] = first_split[0].trim();
        first_split[1] = "$ $";
        first_split[2] = first_split[2].trim();
    }
    let code = code;
    let joined = first_split.join(" ");
    let split: Vec<&str> = joined.split(" ").collect();
    let args: Vec<&&str> = split.iter().skip(1).collect();
    if !args.get(0).ok_or(parse_error(None))?.eq(&&"rules") {
        if args.get(0).ok_or(parse_error(None))?.eq(&&"seen") {
            tx.send(Event::InternalIsRecentlyChecked(
                (***args.get(1).ok_or(parse_error(None))?).to_owned(),
            ))
            .unwrap();
            return Ok(None);
        } else {
            return Err(parse_error(None));
        }
    }

    match args.get(1).ok_or(parse_error(None))? {
        &&"add" => {
            let susp_ip = args.get(3).ok_or(parse_error(None))?.eq(&&"if_susp_ip")
                || args.get(3).ok_or(parse_error(None))?.eq(&&"if_ip_susp");
            if !(args.get(3).ok_or(parse_error(None))?.eq(&&"if") || susp_ip)
                || !args.get(7).ok_or(parse_error(None))?.eq(&&"then")
            {
                return Err(parse_error(None));
            }

            let name: String = (***args.get(2).ok_or(parse_error(None))?).to_owned();

            let criterion_element = args.get(4).ok_or(parse_error(None))?;
            let criterion_check = args.get(5).ok_or(parse_error(None))?;
            let criterion_value: String = (***args.get(6).ok_or(parse_error(None))?).to_owned();

            let criterion = match criterion_element {
                &&"ip" => match criterion_check {
                    &&"equals" => Criterion::IpMatch(Ip(criterion_value)),
                    _ => return Err(parse_error(None)),
                },
                &&"print" => return Err(parse_error(Some("Use lichess print ban instead"))),
                &&"email" => match criterion_check {
                    &&"contains" => Criterion::EmailContains(criterion_value),
                    &&"regex" => Criterion::EmailRegex(Regex::new(&criterion_value)?),
                    _ => return Err(parse_error(None)),
                },
                &&"username" => match criterion_check {
                    &&"contains" => Criterion::UsernameContains(criterion_value),
                    &&"regex" => Criterion::UsernameRegex(Regex::new(&criterion_value)?),
                    _ => return Err(parse_error(None)),
                },
                &&"useragent" => match criterion_check {
                    &&"length-lte" => Criterion::UseragentLengthLte(criterion_value.parse()?),
                    _ => return Err(parse_error(None)),
                },
                &&"lua" => Criterion::Lua(code.to_string()),
                _ => return Err(parse_error(None)),
            };

            let actions: Vec<Action> = args
                .get(8)
                .ok_or(parse_error(None))?
                .split("+")
                .map(|one| match one {
                    "shadowban" => Some(Action::Shadowban),
                    "engine" => Some(Action::EngineMark),
                    "boost" => Some(Action::BoostMark),
                    "ipban" => Some(Action::IpBan),
                    "close" => Some(Action::Close),
                    "alt" => Some(Action::Alt),
                    "panic" => Some(Action::EnableChatPanic),
                    "notify" => Some(Action::NotifySlack),
                    _ => None,
                })
                .flatten()
                .collect();

            if actions.len() != args.get(8).ok_or(parse_error(None))?.split("+").count() {
                return Err(parse_error(None));
            }

            let no_delay = match args.get(9) {
                Some(s) => s == &&"nodelay",
                None => false,
            };

            let should_expire = match args.get(if no_delay { 10 } else { 9 }) {
                Some(s) => {
                    if s == &&"expires" || s == &&"expiry" {
                        true
                    } else {
                        return Err(parse_error(Some(
                            "'expires' or 'expiry' expected, got something else",
                        )));
                    }
                }
                None => false,
            };

            let expiry = if !should_expire {
                None
            } else {
                match args.get(if no_delay { 11 } else { 10 }) {
                    Some(s) => Some(parse_expiry_duration(s)?),
                    None => {
                        return Err(parse_error(Some("expiry time expected, got nothing")));
                    }
                }
            }
            .map(|duration| chrono::Utc::now() + duration);

            let rule = Rule {
                name,
                criterion,
                actions,
                match_count: 0,
                most_recent_caught: vec![],
                no_delay,
                enabled: true,
                susp_ip: susp_ip,
                expiry,
                exp_notification: 0,
                creation_date: chrono::Utc::now(),
                latest_match_date: None,
            };

            tx.send(Event::InternalAddRule { rule }).unwrap();

            Ok(None)
        }
        &&"show" => {
            tx.send(Event::InternalShowRule(
                (***args.get(2).ok_or(parse_error(None))?).to_owned(),
            ))
            .unwrap();

            Ok(None)
        }
        &&"remove" => {
            tx.send(Event::InternalRemoveRule(
                (***args.get(2).ok_or(parse_error(None))?).to_owned(),
            ))
            .unwrap();

            Ok(None)
        }
        &&"disable-re" => {
            tx.send(Event::InternalDisableRules(
                (***args.get(2).ok_or(parse_error(None))?).to_owned(),
            ))
            .unwrap();

            Ok(None)
        }
        &&"enable-re" => {
            tx.send(Event::InternalEnableRules(
                (***args.get(2).ok_or(parse_error(None))?).to_owned(),
            ))
            .unwrap();

            Ok(None)
        }
        &&"renew" => {
            let rule_name = (***args
                .get(2)
                .ok_or(parse_error(Some("Please provide a rule name")))?)
            .to_owned();
            let duration_str = &(***args
                .get(3)
                .ok_or(parse_error(Some("Please provide a new expiry")))?);
            let duration = parse_expiry_duration(duration_str)?;
            tx.send(Event::InternalRenewRule {
                rule: rule_name,
                new_expiry: Utc::now() + duration,
            })
            .unwrap();
            Ok(None)
        }
        &&"list" => {
            tx.send(Event::InternalListRules).unwrap();

            Ok(None)
        }
        &&"test" => {
            let user_unpreprocessed = User::from_json(code)?;
            let Email(email) = user_unpreprocessed.email;
            let email_processed = email
                .split("|")
                .collect::<Vec<&str>>()
                .get(1)
                .ok_or(parse_error(None))?
                .trim_matches('>');
            let user = User {
                username: user_unpreprocessed.username,
                ip: user_unpreprocessed.ip,
                finger_print: user_unpreprocessed.finger_print,
                user_agent: user_unpreprocessed.user_agent,
                email: Email(email_processed.to_string()),
                susp_ip: false,
            };
            tx.send(Event::InternalHypotheticalSignup(user)).unwrap();

            Ok(None)
        }
        _ => Err(parse_error(None)),
    }
}

fn parse_expiry_duration(s: &str) -> Result<Duration, ParseError> {
    let step = s.chars().last().unwrap_or('/');
    let mut arg = s.chars();
    arg.next_back();
    let amount = arg.as_str().parse::<u32>().unwrap_or(0);
    if amount == 0 || (step != 'd' && step != 'w') {
        return Err(parse_error(Some(
            "Invalid expiry date format. Example: `14d`. Supported: `d` (day), `w` (week).",
        )));
    }

    match step {
        'd' => Ok(chrono::Duration::days(amount.into())),
        'w' => Ok(chrono::Duration::weeks(amount.into())),
        _ => unreachable!(),
    }
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
}

fn parse_error(msg: Option<&str>) -> ParseError {
    ParseError {
        message: msg.unwrap_or("Could not parse user command").to_owned(),
    }
}

impl Error for ParseError {
    fn description(&self) -> &str {
        self.message.as_ref()
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl From<std::num::ParseIntError> for ParseError {
    fn from(_: std::num::ParseIntError) -> Self {
        parse_error(Some("Can't parse int"))
    }
}

impl From<regex::Error> for ParseError {
    fn from(err: regex::Error) -> Self {
        parse_error(Some(format!("Invalid regex: {:?}", err).as_ref()))
    }
}

impl From<rlua::Error> for ParseError {
    fn from(_: rlua::Error) -> Self {
        parse_error(Some("Invalid lua"))
    }
}

impl From<serde_json::Error> for ParseError {
    fn from(_: serde_json::Error) -> Self {
        parse_error(Some("Can't (de)serialize"))
    }
}
