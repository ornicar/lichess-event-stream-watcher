use event::{Email, Event, Ip, User};
use regex::Regex;
use serde_json;
use signup::rules::{Action, Criterion, Rule};
use std::error::Error;
use std::sync::mpsc::Sender;

pub fn handle_command(command: String, tx: Sender<Event>) -> Result<Option<String>, ParseError> {
    let cmd = command.clone();
    let parts: Vec<&str> = cmd.split(" ").collect();
    match parts.get(0)? {
        &"status" => handle_status_command(tx.clone()),
        &"signup" => handle_signup_command(command, tx.clone()),
        &"upgrade" => handle_external_command("./upgrade"),
        &"restart" => handle_external_command("./restart"),
        _ => Err(parse_error(None)),
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
        return Err(parse_error(None));
    }

    match args.get(1)? {
        &&"add" => {
            let susp_ip = args.get(3)?.eq(&&"if_susp_ip") || args.get(3)?.eq(&&"if_ip_susp");
            if !(args.get(3)?.eq(&&"if") || susp_ip) || !args.get(7)?.eq(&&"then") {
                return Err(parse_error(None));
            }

            let name: String = (***args.get(2)?).to_owned();

            let criterion_element = args.get(4)?;
            let criterion_check = args.get(5)?;
            let criterion_value: String = (***args.get(6)?).to_owned();

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
                return Err(parse_error(None));
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
                susp_ip: susp_ip,
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
            let user_unpreprocessed = User::from_json(code)?;
            let Email(email) = user_unpreprocessed.email;
            let email_processed = email
                .split("|")
                .collect::<Vec<&str>>()
                .get(1)?
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

fn handle_external_command(command: &str) -> Result<Option<String>, ParseError> {
    println!("handle_external_command called");
    match std::process::Command::new(command).output() {
        Ok(_) => Ok(None),
        Err(_) => Ok(Some(String::from("Failed executing command."))),
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

impl From<std::option::NoneError> for ParseError {
    fn from(_: std::option::NoneError) -> Self {
        parse_error(Some("NoneError"))
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
