use event::{Event, FingerPrint, Ip};
use signup::rules::{Action, Criterion, Rule};
use std::error::Error;
use std::sync::mpsc::Sender;

pub fn handle_command(command: String, tx: Sender<Event>) -> Result<Option<String>, ParseError> {
    let parts: Vec<&str> = command.split(" ").collect();
    match parts.get(0)? {
        &"status" => handle_status_command(tx.clone()),
        &"signup" => handle_signup_command(parts.iter().skip(1).collect(), tx.clone()),
        _ => Err(ParseError {}),
    }
}

fn handle_status_command(tx: Sender<Event>) -> Result<Option<String>, ParseError> {
    tx.send(Event::InternalSlackStatusCommand).unwrap();
    Ok(None)
}

fn handle_signup_command(
    args: Vec<&&str>,
    tx: Sender<Event>,
) -> Result<Option<String>, ParseError> {
    if !args.get(0)?.eq(&&"rules") {
        return Err(ParseError {});
    }

    match args.get(1)? {
        &&"add" => {
            if !args.get(3)?.eq(&&"if") || !args.get(7)?.eq(&&"then") {
                return Err(ParseError {});
            }

            let name: String = (***args.get(2)?).to_owned();

            let criterion_element = args.get(4)?;
            let criterion_check = args.get(5)?;
            let criterion_value: String = (***args.get(6)?).to_owned();

            let criterion = match criterion_element {
                &&"ip" => match criterion_check {
                    &&"equals" => Criterion::IpMatch(Ip(criterion_value)),
                    _ => return Err(ParseError {}),
                },
                &&"print" => match criterion_check {
                    &&"equals" => Criterion::PrintMatch(FingerPrint(criterion_value)),
                    _ => return Err(ParseError {}),
                },
                &&"email" => match criterion_check {
                    &&"contains" => Criterion::EmailContains(criterion_value),
                    _ => return Err(ParseError {}),
                },
                &&"username" => match criterion_check {
                    &&"contains" => Criterion::UsernameContains(criterion_value),
                    _ => return Err(ParseError {}),
                },
                &&"useragent" => match criterion_check {
                    &&"length-lte" => Criterion::UseragentLengthLte(criterion_value.parse()?),
                    _ => return Err(ParseError {}),
                },
                _ => return Err(ParseError {}),
            };

            let actions: Vec<Action> = args
                .get(8)?
                .split("+")
                .map(|one| match one {
                    "shadowban" => Some(Action::Shadowban),
                    "engine" => Some(Action::EngineMark),
                    "boost" => Some(Action::BoostMark),
                    "ipban" => Some(Action::IpBan),
                    "close" => Some(Action::Close),
                    "panic" => Some(Action::EnableChatPanic),
                    "notify" => Some(Action::NotifySlack),
                    _ => None,
                }).flatten()
                .collect();

            if actions.len() != args.get(8)?.split("+").count() {
                return Err(ParseError {});
            }

            let rule = Rule {
                name,
                criterion,
                actions,
                match_count: 0,
                most_recent_caught: vec![],
            };

            tx.send(Event::InternalAddRule { rule }).unwrap();

            Ok(None)
        }
        &&"show" => {
            tx.send(Event::InternalShowRule((***args.get(2)?).to_owned()))
                .unwrap();

            Ok(None)
        }
        &&"remove" => {
            tx.send(Event::InternalRemoveRule((***args.get(2)?).to_owned()))
                .unwrap();

            Ok(None)
        }
        &&"list" => {
            tx.send(Event::InternalListRules).unwrap();

            Ok(None)
        }
        _ => Err(ParseError {}),
    }
}

#[derive(Debug)]
pub struct ParseError;

impl Error for ParseError {
    fn description(&self) -> &str {
        "Could not parse user command"
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Could not parse user command")
    }
}

impl From<std::option::NoneError> for ParseError {
    fn from(_: std::option::NoneError) -> Self {
        ParseError {}
    }
}

impl From<std::num::ParseIntError> for ParseError {
    fn from(_: std::num::ParseIntError) -> Self {
        ParseError {}
    }
}
