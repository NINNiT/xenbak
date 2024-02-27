use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about)]
pub struct XenbakdCli {
    /// Sets a custom config file
    #[clap(short, long)]
    pub config: String,
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}

#[derive(Parser)]
pub enum SubCommand {
    #[clap(name = "daemon", about = "Starts the xenbakd daemon")]
    Daemon(DaemonSubCommand),
    #[clap(name = "run", about = "Runs jobs once")]
    Run(RunSubCommand),
}

#[derive(Parser)]
pub struct DaemonSubCommand {}

#[derive(Parser)]
pub struct RunSubCommand {
    #[clap(short, long)]
    pub jobs: Vec<String>,
}
