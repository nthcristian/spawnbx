use thiserror::Error;

use crate::model::CommandInvocation;

pub trait InvocationParser {
    fn parse(&self) -> Result<CommandInvocation, InvocationParseError>;
}

#[derive(Debug, Error)]
pub enum InvocationParseError {
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    #[error("unknown option: {0}")]
    UnknownOption(String),
    #[error("invalid command-line value: {0}")]
    InvalidArgumentValue(String),
    #[error("failed to parse command-line arguments")]
    ParseFailed {
        #[source]
        source: anyhow::Error,
    },
}
