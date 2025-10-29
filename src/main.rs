use clap::{Parser, Subcommand};

use crate::error::PyeCliError;

pub mod error;
pub mod pye_api;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Parser)]
pub struct ValidatorLockupManagerArgs {
    #[arg(long, env)]
    pye_api_key: String,
    #[arg(long, env, default_value = "https://gwtgzlzfnztqhiulhgtm.supabase.co")]
    api_url: String,
    #[arg(long, env)]
    epoch: u64,
    #[arg(long, env)]
    keypair_path: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Will run the excess rewards stuff for all pye_accounts owned by a validator
    ValidatorLockupManager {
        #[command(flatten)]
        args: ValidatorLockupManagerArgs,
    },
}

#[tokio::main]
async fn main() -> Result<(), PyeCliError> {
    println!("Hello, world!");
    let cli = Cli::parse();
    match cli.command {
        Commands::ValidatorLockupManager { args } => {
            // TODO: Wait for next epoch
            crate::pye_api::fetch_lockup_rewards(&args.api_url, &args.pye_api_key, args.epoch)
                .await?;
            // TODO: Determine amount of payments that need to be made for each validator
            // TODO: Load in keypair
            // TODO: Stack payment instructions and send transactions
        }
    }

    Ok(())
}
