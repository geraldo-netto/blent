//! One action's identity travels with its original receiver. Losing the receiver
//! never settles delegated native work; feedback must keep that uncertainty clear.
use crate::FeedbackArea;
use std::sync::mpsc::Receiver;

pub(super) struct Job {
    pub receiver: Receiver<String>,
    pub area: FeedbackArea,
    operation: String,
}

impl Job {
    pub fn new(area: FeedbackArea, operation: impl Into<String>, receiver: Receiver<String>) -> Self {
        Self { receiver, area, operation: operation.into() }
    }

    pub fn completed(&self, message: &str) -> String {
        format!("{}: {message}", self.operation)
    }

    pub fn lost_result(&self) -> String {
        format!("{} returned no result; its effect is unconfirmed. {}", self.operation, recovery(self.area))
    }
}

fn recovery(area: FeedbackArea) -> &'static str {
    match area {
        FeedbackArea::Display => "Check Tablets and Connections for the latest state before retrying.",
        FeedbackArea::Setup => "Check General for missing prerequisites or permission errors before retrying.",
        FeedbackArea::Settings => "Check saved preferences in General before retrying.",
    }
}
