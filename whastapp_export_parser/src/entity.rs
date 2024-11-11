use chrono::NaiveDateTime;

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub date_time: NaiveDateTime,
    pub message_type: MessageType,
}

impl Message {
    pub fn system_message(date_time: NaiveDateTime, text: String) -> Message {
        Message {
            date_time,
            message_type: MessageType::System(text),
        }
    }

    pub fn user_message(date_time: NaiveDateTime, sender: String, body: MessageBody) -> Message {
        Message {
            date_time,
            message_type: MessageType::User { sender, body },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageType {
    System(String),
    User { sender: String, body: MessageBody },
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageBody {
    Text {
        text: Vec<String>,
        edited: bool,
    },
    Poll {
        title: String,
        options: Vec<PollOption>,
    },
    Attachment(String),
    Omitted,
    // deleted ?
    Location {
        lat: f32,
        long: f32,
    }, // todo test live location
    Event {
        // todo more types
        title: String,
        description: String,
        start_time: NaiveDateTime,
        end_time: Option<NaiveDateTime>,
        cancelled: bool,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PollOption {
    option: String,
    votes: u32,
}
