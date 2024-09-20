use thiserror::Error;

#[derive(Debug, Error)]
pub enum XApiCliError {
    #[error("Error: {0}")]
    GenericError(String),
    #[error("Command could not be executed: {0}")]
    CommandExecutionError(#[from] tokio::io::Error),
    #[error("xe cli-command failed: {0}")]
    CommandFailed(String),
    #[error("Failed to create snapshot: {0}")]
    SnapshotFailure(String),
    #[error("Failed to parse cli stdout to struct: {0}")]
    CliParseError(String),
    #[error("Failed to parse xen timestamp")]
    TimestampParseError(#[from] chrono::ParseError),
}
