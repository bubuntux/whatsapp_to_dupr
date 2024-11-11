use chrono::{NaiveDate, NaiveDateTime};
use io::{BufReader, Lines};
use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use std::fs::{File, OpenOptions};
use std::io::{self, BufRead};
use std::io::{BufWriter, Write};
use std::iter::{Flatten, Peekable};
use std::path::Path;

fn main() {
    let mut out = get_output();
    let conf = get_config();

    let players = conf.players;

    Parser::new("./chat.txt")
        .skip_while(|message| {
            message
                .date_time
                .lt(&NaiveDate::parse_from_str("2024-11-07", "%Y-%m-%d")
                    .unwrap()
                    .into())
        })
        .filter(|message| match &message.body {
            MessageBody::Text(text) => GAME.is_match(&text),
            MessageBody::MultiText(texts) => texts.iter().any(|text| GAME.is_match(&text)),
            _ => false,
        })
        .flat_map(|message| match message.body {
            MessageBody::Text(text) => vec![process_message(
                &players,
                &message.date_time,
                &message.sender,
                &text,
            )],
            MessageBody::MultiText(texts) => texts
                .iter()
                .map(|text| process_message(&players, &message.date_time, &message.sender, &text))
                .collect(),
            _ => vec![],
        })
        .for_each(|line| {
            if let Some(s) = line {
                writeln!(out, "{}", s).expect("write line")
            }
        });
}

fn get_config() -> Config {
    let config_file = File::open("./config.json").expect("Open the config json file");
    let mut config: Config = serde_json::from_reader(config_file).expect("Parse the config file");
    config.players.iter_mut().for_each(|player| {
        if let Some(waid) = &player.whatsapp_id {
            if !waid.starts_with("@") {
                player.whatsapp_id = Some(format!("@{}", waid))
            }
        }

        let new_aliases: Vec<String> = player
            .aliases
            .iter()
            .filter(|alias| !alias.starts_with("@"))
            .map(|alias| format!("@{}", alias))
            .collect();
        player.aliases.extend(new_aliases);
        player.aliases.sort();
    });
    config
}

fn get_output() -> BufWriter<File> {
    let out = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open("./out.csv")
        .expect("Open the output file");
    BufWriter::new(out)
}

fn get_player<'a>(
    club: &'a Vec<Player>,
    other_dupr_ids: &Vec<&Player>,
    sender: &String,
    message: &mut String,
) -> Option<&'a Player> {
    club.iter().find(|player: &&Player| {
        if other_dupr_ids
            .iter()
            .any(|p: &&Player| p.dupr_id.eq(&player.dupr_id))
        {
            return false;
        }
        if (message.starts_with("@me")
            || message.starts_with("@ne")
            || message.starts_with("@mw")
            || message.starts_with("@yo"))
            && player.name.eq(sender)
        {
            *message = message[3..].trim().to_string();
            return true;
        }
        if let Some(whatsapp_id) = &player.whatsapp_id {
            if message.starts_with(whatsapp_id) {
                *message = message[whatsapp_id.len()..].trim().to_string();
                return true;
            }
        }

        let alia = player
            .aliases
            .iter()
            .find(|alias| message.starts_with(*alias));

        if alia.is_some() {
            *message = message[alia.unwrap().len()..].trim().to_string();
            true
        } else {
            false
        }
    })
}

fn process_message(
    players: &Vec<Player>,
    date_time: &NaiveDateTime,
    sender: &String,
    message: &String,
) -> Option<String> {
    let mut_message = message
        .to_lowercase()
        .replace("vs", " ")
        .replace("va", " ")
        .replace("@ me", "@me")
        .replace(" @ ", " ")
        .replace(" y ", " ")
        .replace("\u{2068}", "")
        .replace("<this message was edited>", "");
    let mut_message = CLEAN.replace_all(&mut_message, " ").to_string();
    let mut_message = WHITES.replace_all(&mut_message, " ").trim().to_string();
    let mut mut_message = [" ", mut_message.as_str()].join("");

    let (score_a, score_b) = extract_scores(&mut mut_message);
    if score_a == score_b {
        println!(
            "{:#?}",
            format!("Bad scores - {:?},{:?},{:?}", date_time, sender, message)
        );
        return None;
    }

    let mut other_dupr_ids: Vec<&Player> = Vec::new();

    for i in 1..5 {
        let player = get_player(players, &other_dupr_ids, sender, &mut mut_message);
        if player.is_none() {
            println!(
                "{:#?}",
                format!(
                    "Bad player {:?} - {:?},{:?},{:?}",
                    i, date_time, sender, message
                )
            );
            return None;
        }
        let player = player.unwrap();
        other_dupr_ids.push(player);
    }

    Some(format!(
        ",,,D,,{},{},{},,{},{},,{},{},,{},{},,,{},{}",
        date_time.format("%Y-%m-%d"),
        other_dupr_ids[0].dupr_name,
        other_dupr_ids[0].dupr_id,
        other_dupr_ids[1].dupr_name,
        other_dupr_ids[1].dupr_id,
        other_dupr_ids[2].dupr_name,
        other_dupr_ids[2].dupr_id,
        other_dupr_ids[3].dupr_name,
        other_dupr_ids[3].dupr_id,
        score_a,
        score_b
    ))
}

fn extract_scores(message: &mut String) -> (u8, u8) {
    let games = Regex::new(r"\s+(\d{1,2}).*\s+(\d{1,2})").unwrap();
    let scores = match games.captures(&message) {
        None => (0, 0),
        Some(caps) => (caps[1].parse().unwrap(), caps[2].parse().unwrap()),
    };
    *message = message
        .replace(format!(" {:?}", scores.0).as_str(), " ")
        .trim()
        .to_string();
    *message = message
        .replace(format!(" {:?}", scores.1).as_str(), " ")
        .trim()
        .to_string();
    scores
}

#[derive(Deserialize, Debug)]
struct Config {
    pub players: Vec<Player>,
}

#[derive(Deserialize, Debug)]
struct Player {
    pub name: String,
    pub dupr_id: String,
    pub dupr_name: String,
    pub whatsapp_id: Option<String>,
    pub aliases: Vec<String>,
}

/// Move to other file

fn read_lines<P>(filename: P) -> io::Result<Lines<BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(BufReader::new(file).lines())
}

#[derive(Debug)]
enum MessageBody {
    Text(String),
    MultiText(Vec<String>),
    Poll,
}

#[derive(Debug)]
struct Message {
    pub date_time: NaiveDateTime,
    pub sender: String,
    pub body: MessageBody,
}

struct Parser {
    pub lines: Peekable<Flatten<Lines<BufReader<File>>>>,
}

impl Parser {
    fn new(filename: &str) -> Parser {
        Parser {
            lines: read_lines(filename).unwrap().flatten().peekable(),
        }
    }
}

lazy_static! {
    static ref MESSAGE: Regex =
        Regex::new(r"(?<date_time>^\d+\/\d+\/\d+,\s\d+:\d+\s\S+) - ").unwrap();
    static ref MESSAGE_WITH_BODY: Regex =
        Regex::new(r"(?<date_time>^\d+\/\d+\/\d+,\s\d+:\d+\s\S+) - (?<sender>.*: )(?<body>.*)")
            .unwrap();
    static ref GAME: Regex = Regex::new(r"(\d{1,2})[^:@0-9]+(\d{1,2})").unwrap();
    static ref CLEAN: Regex = Regex::new(r"[_\-.,&/=]").unwrap();
    static ref WHITES: Regex = Regex::new(r"\s+").unwrap();
}

impl Iterator for Parser {
    type Item = Message;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(s) = self.lines.next() {
            if !MESSAGE_WITH_BODY.is_match(&s) {
                // println!("############ignoring {:#?}", s);
                continue;
            }
            let captures = MESSAGE_WITH_BODY.captures(&s)?;
            let date_time = &captures["date_time"];
            let sender = &captures["sender"].replace(": ", "");
            let body = &captures["body"];
            let mut message = Message {
                date_time: NaiveDateTime::parse_from_str(date_time, "%D, %l:%M %p").unwrap(),
                sender: sender.to_string(),
                body: MessageBody::Text(body.to_string()),
            };
            if body == "POLL:" {
                while let Some(next_line) = self.lines.peek() {
                    if MESSAGE_WITH_BODY.is_match(next_line) {
                        break;
                    } else {
                        self.lines.next()?;
                    }
                }
                message.body = MessageBody::Poll;
            }
            if let Some(next_line) = self.lines.peek() {
                if !MESSAGE.is_match(next_line) {
                    let mut bv = vec![body.to_string()];
                    bv.push(self.lines.next()?);
                    while let Some(next_line) = self.lines.peek() {
                        if MESSAGE.is_match(next_line) {
                            break;
                        } else {
                            bv.push(self.lines.next()?)
                        }
                    }
                    message.body = MessageBody::MultiText(bv);
                }
            }
            return Some(message);
        }
        None
    }
}
