use event::{Event, FingerPrint, Ip};
use signup::rules::{Action, Criterion, Rule};
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
    if !args.get(0)?.eq(&&"rules")
        || !args.get(1)?.eq(&&"add")
        || !args.get(3)?.eq(&&"if")
        || !args.get(7)?.eq(&&"then")
    {
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

    let action = match args.get(8)? {
        &&"shadowban" => Action::Shadowban,
        &&"engine" => Action::EngineMark,
        &&"boost" => Action::BoostMark,
        &&"ipban" => Action::IpBan,
        &&"close" => Action::Close,
        &&"panic" => Action::EnableChatPanic,
        &&"notify" => Action::NotifySlack,
        _ => return Err(ParseError {}),
    };

    let rule = Rule {
        name,
        criterion,
        action,
    };

    tx.send(Event::InternalAddRule { rule }).unwrap();

    Ok("Rule added!".to_owned())
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
