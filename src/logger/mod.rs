//! In-memory log shown in the editor console.
//!
//! Every subsystem reports through the global [`LOGGER`]. Messages are kept
//! in a bounded buffer; a message identical to the previous one only bumps
//! that message's repeat count, so a script failing every frame doesn't
//! flood the console.

use chrono::Local;
use std::collections::VecDeque;
use std::sync::{LazyLock, Mutex};

/// Messages kept for the console; older ones are dropped.
const MAX_CONSOLE_MESSAGES: usize = 1000;

#[derive(Clone, Debug)]
pub struct ConsoleMessage {
    pub text: String,
    /// When the message (or its latest repeat) was logged.
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub message_type: ConsoleMessageType,
    /// How many times in a row this message was logged.
    pub count: u32,
}

/// Severity, ordered from least to most severe (`Debug` sorts last and is
/// shown separately by the console).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsoleMessageType {
    Info,
    Warning,
    Error,
    Debug,
}

impl ConsoleMessage {
    pub fn new(text: String, message_type: ConsoleMessageType) -> Self {
        Self {
            text,
            timestamp: Local::now(),
            message_type,
            count: 1,
        }
    }
}

#[derive(Default)]
struct Log {
    messages: VecDeque<ConsoleMessage>,
    /// Incremented on every change, so readers can skip unchanged logs
    revision: u64,
}

pub struct Logger {
    log: Mutex<Log>,
}

impl Logger {
    fn new() -> Self {
        Self {
            log: Mutex::new(Log::default()),
        }
    }

    fn log_message(&self, text: String, message_type: ConsoleMessageType) {
        let mut log = self.log.lock().unwrap_or_else(|e| e.into_inner());
        log.revision += 1;
        if let Some(last) = log.messages.back_mut()
            && last.text == text
            && last.message_type == message_type
        {
            last.count += 1;
            last.timestamp = Local::now();
            return;
        }
        log.messages
            .push_back(ConsoleMessage::new(text, message_type));
        if log.messages.len() > MAX_CONSOLE_MESSAGES {
            log.messages.pop_front();
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

    /// A copy of the buffered messages, oldest first.
    pub fn get_console_messages(&self) -> Vec<ConsoleMessage> {
        let log = self.log.lock().unwrap_or_else(|e| e.into_inner());
        log.messages.iter().cloned().collect()
    }

    /// Changes whenever a message is logged; cheap to poll every frame.
    pub fn revision(&self) -> u64 {
        self.log.lock().unwrap_or_else(|e| e.into_inner()).revision
    }

    /// Remove all messages.
    pub fn clear(&self) {
        let mut log = self.log.lock().unwrap_or_else(|e| e.into_inner());
        log.messages.clear();
        log.revision += 1;
    }
}

pub static LOGGER: LazyLock<Logger> = LazyLock::new(Logger::new);
