use clap::Parser;
use anyhow::Result;

mod commands;

/// Official SexOS userspace scaffolding CLI
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser)]
enum Commands {
    /// Create a new SexOS application
    New(commands::new::NewArgs),
    /// Port an application from another OS
    PortFrom(commands::port_from::PortFromArgs),
    /// Upgrade a core component
    Upgrade(commands::upgrade::UpgradeArgs),
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New(args) => commands::new::handle_new(args),
        Commands::PortFrom(args) => commands::port_from::handle_port_from(args),
        Commands::Upgrade(args) => commands::upgrade::handle_upgrade(args),
    }
}
