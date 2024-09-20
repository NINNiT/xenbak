use clap::{Arg, Parser};

#[derive(Parser)]
#[command(version, about, long_about)]
pub struct XenbakdCli {
    /// Sets a custom config file
    #[clap(short, long)]
    pub config: String,
}
