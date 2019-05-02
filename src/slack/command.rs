use event::{Event, FingerPrint, Ip, User};
use regex::Regex;
use signup::rules::{Action, Criterion, Rule};
use std::error::Error;
use std::sync::mpsc::Sender;
use serde_json;

pub fn handle_command(command: String, tx: Sender<Event>) -> Result<Option<String>, ParseError> {
    let cmd = command.clone();
    let parts: Vec<&str> = cmd.split(" ").collect();
    match parts.get(0)? {
        &"status" => handle_status_command(tx.clone()),
        &"signup" => handle_signup_command(command, tx.clone()),
        &"upgrade" => handle_external_command("./upgrade"),
        &"restart" => handle_external_command("./restart"),
        _ => Err(ParseError {}),
    }
}

fn handle_status_command(tx: Sender<Event>) -> Result<Option<String>, ParseError> {
    tx.send(Event::InternalSlackStatusCommand).unwrap();
    Ok(None)
}

fn handle_signup_command(command: String, tx: Sender<Event>) -> Result<Option<String>, ParseError> {
    let mut first_split: Vec<&str> = command.split("`").collect();
    let mut code = "";
    if first_split.len() > 2 {
        code = first_split.get(1)?.clone();
        first_split[0] = first_split[0].trim();
        first_split[1] = "$ $";
        first_split[2] = first_split[2].trim();
    }
    let code = code;
    let joined = first_split.join(" ");
    let split: Vec<&str> = joined.split(" ").collect();
    let args: Vec<&&str> = split.iter().skip(1).collect();
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
                    &&"regex" => Criterion::EmailRegex(Regex::new(&criterion_value)?),
                    _ => return Err(ParseError {}),
                },
                &&"username" => match criterion_check {
                    &&"contains" => Criterion::UsernameContains(criterion_value),
                    &&"regex" => Criterion::UsernameRegex(Regex::new(&criterion_value)?),
                    _ => return Err(ParseError {}),
                },
                &&"useragent" => match criterion_check {
                    &&"length-lte" => Criterion::UseragentLengthLte(criterion_value.parse()?),
                    _ => return Err(ParseError {}),
                },
                &&"lua" => Criterion::Lua(code.to_string()),
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
                })
                .flatten()
                .collect();

            if actions.len() != args.get(8)?.split("+").count() {
                return Err(ParseError {});
            }

            let no_delay = match args.get(9) {
                Some(s) => s == &&"nodelay",
                None => false,
            };

            let rule = Rule {
                name,
                criterion,
                actions,
                match_count: 0,
                most_recent_caught: vec![],
                no_delay,
                enabled: true,
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
        &&"disable-re" => {
            tx.send(Event::InternalDisableRules((***args.get(2)?).to_owned()))
                .unwrap();

            Ok(None)
        }
        &&"enable-re" => {
            tx.send(Event::InternalEnableRules((***args.get(2)?).to_owned()))
                .unwrap();

            Ok(None)
        }
        &&"list" => {
            tx.send(Event::InternalListRules).unwrap();

            Ok(None)
        }
        &&"test" => {
            tx.send(Event::InternalHypotheticalSignup(User::from_json(code)?)).unwrap();

            Ok(None)
        }
        _ => Err(ParseError {}),
    }
}

fn handle_external_command(command: &str) -> Result<Option<String>, ParseError> {
    println!("handle_external_command called");
    match std::process::Command::new(command).output() {
        Ok(_) => Ok(None),
        Err(_) => Ok(Some(String::from("Failed executing command."))),
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

impl From<regex::Error> for ParseError {
    fn from(_: regex::Error) -> Self {
        ParseError {}
    }
}

impl From<rlua::Error> for ParseError {
    fn from(_: rlua::Error) -> Self {
        ParseError {}
    }
}

impl From<serde_json::Error> for ParseError {
    fn from(_: serde_json::Error) -> Self {
        ParseError {}
    }
}