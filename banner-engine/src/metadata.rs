use const_format::concatcp;

use crate::Event;

// Metadata became a way of evolving the usage; without having to care
// too much about blowing up the implementations between versions.
// It might work.. It might not... Truth of the matter is, I don't know
// all the stuff I might want to pass around about an event yet.
#[derive(Debug, Clone, PartialEq)]
pub struct Metadata {
    key: String,
    value: String,
}

impl Metadata {
    pub fn new(key: &str, value: &str) -> Self {
        if key.starts_with(&BANNER_METADATA_PREFIX) {
            panic!(
                "You may not use banner annotations directly. This is a reserved prefix. {}",
                BANNER_METADATA_PREFIX
            )
        };
        Self {
            key: key.to_string(),
            value: value.to_string(),
        }
    }

    pub(crate) fn new_banner_error(value: &str) -> Self {
        Self {
            key: ERROR_TAG.to_owned(),
            value: value.to_string(),
        }
    }

    pub(crate) fn new_banner_task(value: &str) -> Self {
        Self {
            key: TASK_TAG.to_owned(),
            value: value.to_string(),
        }
    }

    pub(crate) fn new_banner_job(value: &str) -> Self {
        Self {
            key: JOB_TAG.to_owned(),
            value: value.to_string(),
        }
    }

    pub(crate) fn new_banner_pipeline(value: &str) -> Self {
        Self {
            key: PIPELINE_TAG.to_owned(),
            value: value.to_string(),
        }
    }

    pub(crate) fn new_banner_event(value: &Event) -> Self {
        Self {
            key: EVENT_TAG.to_owned(),
            value: value.to_string(),
        }
    }

    pub fn key(&self) -> &str {
        self.key.as_ref()
    }

    pub fn value(&self) -> &str {
        self.value.as_ref()
    }

    pub(crate) fn new_log_message(message: &str) -> Metadata {
        Self {
            key: LOG_MESSAGE_TAG.to_owned(),
            value: message.to_string(),
        }
    }

    pub(crate) fn new_banner_description(description: &str) -> Metadata {
        Self {
            key: DESCRIPTION_TAG.to_owned(),
            value: description.to_string(),
        }
    }
}

impl From<&str> for Metadata {
    fn from(value: &str) -> Self {
        todo!()
    }
}

pub const BANNER_METADATA_PREFIX: &str = "banner.dev";
pub const PIPELINE_TAG: &str = concatcp!(BANNER_METADATA_PREFIX, "/", "pipeline");
pub const JOB_TAG: &str = concatcp!(BANNER_METADATA_PREFIX, "/", "job");
pub const TASK_TAG: &str = concatcp!(BANNER_METADATA_PREFIX, "/", "task");
pub const EVENT_TAG: &str = concatcp!(BANNER_METADATA_PREFIX, "/", "event");
pub const ERROR_TAG: &str = concatcp!(BANNER_METADATA_PREFIX, "/", "error");
pub const MATCHING_TAG: &str = concatcp!(BANNER_METADATA_PREFIX, "/", "matching");
pub const LOG_MESSAGE_TAG: &str = concatcp!(BANNER_METADATA_PREFIX, "/", "log_message");
pub const DESCRIPTION_TAG: &str = concatcp!(BANNER_METADATA_PREFIX, "/", "description");
