use std::sync::Mutex;
use once_cell::sync::Lazy;
use chrono::Local;

const MAX_CONSOLE_MESSAGES: usize = 1000;
const MAX_STORED_MESSAGES: usize = 50000;

#[derive(Clone, Debug)]
pub struct ConsoleMessage {
    pub text: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub message_type: ConsoleMessageType,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsoleMessageType {
    Info,    // 0
    Warning, // 1
    Error,   // 2
    Debug,
}

impl ConsoleMessage {
    pub fn new(text: String, message_type: ConsoleMessageType) -> Self {
        Self {
            text,
            timestamp: Local::now(),
            message_type,
        }
    }
}

pub struct Logger {
    console_messages: Mutex<Vec<ConsoleMessage>>,
    stored_messages: Mutex<Vec<ConsoleMessage>>,
}

impl Logger {
    fn new() -> Self {
        Self {
            console_messages: Mutex::new(Vec::new()),
            stored_messages: Mutex::new(Vec::new()),
        }
    }

    fn log_message(&self, message: String, message_type: ConsoleMessageType) {
        let new_message = ConsoleMessage::new(message, message_type);

        {
            let mut console_logger = self.console_messages.lock().unwrap();
            console_logger.push(new_message.clone());
            if console_logger.len() > MAX_CONSOLE_MESSAGES {
                let excess = console_logger.len() - MAX_CONSOLE_MESSAGES;
                console_logger.drain(0..excess);
            }
        }

        {
            let mut stored_logger = self.stored_messages.lock().unwrap();
            stored_logger.push(new_message);
            if stored_logger.len() > MAX_STORED_MESSAGES {
                let excess = stored_logger.len() - MAX_STORED_MESSAGES;
                stored_logger.drain(0..excess);
            }
        }
    }

    pub fn info(&self, message: impl Into<String>) {
        self.log_message(message.into(), ConsoleMessageType::Info);
    }

    pub fn warning(&self, message: impl Into<String>) {
        self.log_message(message.into(), ConsoleMessageType::Warning);
    }

    pub fn error(&self, message: impl Into<String>) {
        self.log_message(message.into(), ConsoleMessageType::Error);
    }

    pub fn debug(&self, message: impl Into<String>) {
        self.log_message(message.into(), ConsoleMessageType::Debug);
    }

    pub fn get_console_messages(&self) -> Vec<ConsoleMessage> {
        self.console_messages.lock().unwrap().clone()
    }

    pub fn get_stored_messages(&self) -> Vec<ConsoleMessage> {
        self.stored_messages.lock().unwrap().clone()
    }
}

pub static LOGGER: Lazy<Logger> = Lazy::new(Logger::new);