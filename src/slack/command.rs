use event::Event;
use std::error::Error;
use std::sync::mpsc::Sender;

pub fn handle_command(command: String, tx: Sender<Event>) -> Result<String, ParseError> {
    let parts: Vec<&str> = command.split(" ").collect();
    match parts.get(0)? {
        &"status" => Ok("I'm alive".to_owned()),
        &"signup" => handle_signup_command(parts.iter().skip(1).collect(), tx.clone()),
        _ => Err(ParseError {}),
    }
}

fn handle_signup_command(args: Vec<&&str>, tx: Sender<Event>) -> Result<String, ParseError> {
    Ok("Not yet implemented".to_owned())
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
