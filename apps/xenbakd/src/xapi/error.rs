use thiserror::Error;

#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum XApiParseError {
    #[error("Failed to parse xen timestamp")]
    TimestampParseError(#[from] chrono::ParseError),
    #[error("Failed to parse xen output")]
    GenericParseError(String),
    #[error("Empty output")]
    EmptyOutput,
}

#[derive(Debug, Error)]
pub enum XApiCliError {
    #[error("Failed to create snapshot: {0}")]
    SnapshotFailure(String),
    #[error("'xe' command could not be executed: {0}")]
    CommandExecutionError(#[from] tokio::io::Error),
    #[error("'xe' cli-command failed: {0}")]
    CommandFailed(String),
    #[error("Failed to parse cli stdout to struct: {0}")]
    XApiParseError(#[from] XApiParseError),
}

#[derive(Error, Debug)]
pub enum XApiError {
    #[error("CLI Error: {0}")]
    XApiCliError(#[from] XApiCliError),
}
