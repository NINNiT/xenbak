use chrono::Utc;

use crate::error::XApiCliError;

pub mod vm;

pub trait FromCliOutput {
    fn from_cli_output(output: &str) -> Result<Self, XApiCliError>
    where
        Self: Sized;
}

pub type Uuid = String;
impl FromCliOutput for Uuid {
    fn from_cli_output(output: &str) -> Result<Self, XApiCliError> {
        Ok(clean_stdout(output))
    }
}

pub type Uuids = Vec<String>;
impl FromCliOutput for Uuids {
    fn from_cli_output(output: &str) -> Result<Self, XApiCliError> {
        Ok(clean_stdout(output)
            .split(",")
            .map(|s| s.to_string())
            .collect())
    }
}

// remove leading and trailing whitespace, newlines, and carriage returns
pub fn clean_stdout(stdout: &str) -> String {
    stdout.trim().replace("\n", "").replace("\r", "")
}

pub fn parse_timestamp(timestamp: &str) -> Result<chrono::DateTime<chrono::Utc>, XApiCliError> {
    let naive = chrono::NaiveDateTime::parse_from_str(timestamp, "%Y%m%dT%H:%M:%S%Z")?;
    let utc = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, Utc);
    Ok(utc)
}
