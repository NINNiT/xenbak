use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about)]
pub struct XenbakdCli {
    /// Sets a custom config file
    #[clap(short, long)]
    pub config: String,
    #[clap(subcommand)]
    pub subcmd: Option<SubCommand>,
}

#[derive(Parser)]
pub enum SubCommand {
    #[clap(name = "daemon", about = "Starts the xenbakd daemon")]
    Daemon(DaemonSubCommand),
    #[clap(name = "init-storage", about = "Initializes storage backends")]
    InitStorage(InitalizeStorageSubCommand),
    #[clap(name = "dry-run", about = "Runs jobs in dry-run mode")]
    DryRun(DryRunSubCommand),
    #[clap(name = "run", about = "Runs jobs once")]
    Run(RunSubCommand),
}

#[derive(Parser)]
pub struct DaemonSubCommand {}

#[derive(Parser)]
pub struct InitalizeStorageSubCommand {
    #[clap(short, long)]
    pub storages: Option<Vec<String>>,
}

#[derive(Parser)]
pub struct RunSubCommand {
    #[clap(short, long)]
    pub jobs: Option<Vec<String>>,
}

#[derive(Parser)]
pub struct DryRunSubCommand {
    #[clap(short, long)]
    pub jobs: Option<Vec<String>>,
}
