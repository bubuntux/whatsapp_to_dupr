use crate::entity::*;
use chrono::NaiveDateTime;
use lazy_static::lazy_static;
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, Cursor, Read};

struct MessageParser<T> {
    //  pub lines: Flatten<Lines<Box<dyn Read>>>,
    pub lines: Cursor<T>,
}

impl<T> MessageParser<T> {
    /*  fn new_from_file(filename: &str) -> MessageParser {
        MessageParser {
          /*  lines: BufReader::new(File::open(filename))
                .lines()
                .flatten()
                .peekable(),*/
        }
    }*/

    fn new_from_file(filename: &str) -> MessageParser<File> {
        MessageParser {
            lines: Cursor::new(File::open(filename).unwrap()),
        }
    }

    fn new_from_string(s: &str) -> MessageParser<&str> {
        MessageParser {
            lines: Cursor::new(s),
            //  lines: Box::new(Cursor::new(s)).lines().flatten().peekable(),
        }
    }
}

lazy_static! {
    static ref DATE_TIME: Regex =
        Regex::new(r"(?<date_time>^\d+\/\d+\/\d+,\s\d+:\d+\s\S+) - ").unwrap();
    static ref MESSAGE_BODY: Regex = Regex::new(r"(?<sender>.*): (?<body>.*)").unwrap();
}

impl <T: Read> Iterator for MessageParser<T> {
    type Item = Message;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(s) = self.lines.next() {
            let so = DATE_TIME.captures(&s);
            if so.is_none() {
                return None; //todo log error ?
            }
            let date_time =
                NaiveDateTime::parse_from_str(&so?["date_time"].to_string(), "%D, %l:%M %p")
                    .unwrap();
            let s = DATE_TIME.replace(&s, "").to_string();

            let so = MESSAGE_BODY.captures(&s);
            if so.is_none() {
                return Some(Message::system_message(date_time, s));
            }

            let sender = so?["sender"].to_string();
            let body = so?["body"].to_string();

            if body == "<Media omitted>" {
                return Some(Message::user_message(
                    date_time,
                    sender,
                    MessageBody::Omitted,
                ));
            }
            if body == "POLL:" {
                return Some(Message::user_message(date_time, sender, MessageBody::Poll));
                /* while let Some(next_line) = self.lines.peek() {
                    if crate::MESSAGE_WITH_BODY.is_match(next_line) {
                        break;
                    } else {
                        self.lines.next()?;
                    }
                }
                message.body = MessageBody::Poll;*/
            }
            if body.starts_with("EVENT: ") {
                return Some(Message::user_message(date_time, sender, MessageBody::Event));
            }
            if body.starts_with("location: https://maps.google.com/?q=") {
                //todo regex
                return Some(Message::user_message(
                    date_time,
                    sender,
                    MessageBody::Location,
                ));
            }
            if body.ends_with(" (file attached)") {
                return Some(Message::user_message(
                    date_time,
                    sender,
                    MessageBody::Attachment("".to_string()),
                ));
            }

            let mut text = vec![body];
            while let Some(next_line) = self.lines.peek() {
                if !DATE_TIME.is_match(next_line) {
                    text.push(self.lines.next()?);
                }
            }

            return Some(Message::user_message(
                date_time,
                sender,
                MessageBody::Text {
                    text,
                    edited: false,
                },
            ));
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::entity::{MessageBody, MessageType};
    use crate::parser::MessageParser;

    #[test]
    fn parse_text() {
        let mut p = MessageParser::new_from_string(
            "9/20/24, 2:15 PM - Julio Guti: Hello\n
                World",
        );
        let m = p.next().unwrap();
        match m.message_type {
            MessageType::System(_) => {}
            MessageType::User { sender, body } => match body {
                MessageBody::Text { text, edited } => {
                    assert_eq!(text, vec!["Hello", "World"]);
                }
                MessageBody::Poll { .. } => {}
                MessageBody::Attachment(_) => {}
                MessageBody::Omitted => {}
                MessageBody::Location { .. } => {}
                MessageBody::Event { .. } => {}
            },
        }
    }
}
