use std::time::Duration;

use clap::{Parser, Subcommand};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::read_keypair_file;
use tracing::{Level, info};
use tracing_subscriber::EnvFilter;

use crate::error::PyeCliError;

pub mod error;
pub mod pye_api;
pub mod utils;

#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Parser)]
pub struct CommonHandlerArgs {
    /// RPC Endpoint
    #[arg(
        short,
        long,
        env,
        default_value = "https://api.mainnet-beta.solana.com"
    )]
    rpc_url: String,
    /// Path to payer keypair
    #[arg(short, long, env)]
    payer: String,
    #[arg(long, env)]
    pye_api_key: String,
    #[arg(long, env, default_value = "https://tfrickmnrfyjkvjhmuik.supabase.co")]
    api_url: String,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Will run the excess rewards stuff for all pye_accounts owned by a validator
    ValidatorLockupManager {
        #[command(flatten)]
        args: CommonHandlerArgs,
        /// The wait time (in secs) between epoch change checks
        #[arg(long, env, default_value = "60")]
        cycle_secs: u64,
    },
}

#[tokio::main]
async fn main() -> Result<(), PyeCliError> {
    dotenvy::dotenv().ok();
    let level = std::env::var("RUST_LOG").unwrap_or(Level::INFO.to_string());
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::new(level))
        // this needs to be set to remove duplicated information in the log.
        .with_current_span(false)
        // this needs to be set to false, otherwise ANSI color codes will
        // show up in a confusing manner in CloudWatch logs.
        .with_ansi(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        // remove the name of the function from every log entry
        .with_target(false)
        .init();

    let cli = Cli::parse();
    match cli.command {
        Commands::ValidatorLockupManager { args, cycle_secs } => {
            let payer = read_keypair_file(&args.payer)
                .map_err(|err| PyeCliError::ReadKeypairError(err.to_string()))?;

            let rpc_client = RpcClient::new(args.rpc_url.clone());

            loop {
                let handle_payments_res = crate::utils::handle_payments_to_be_sent(
                    &rpc_client,
                    &args.api_url,
                    &args.pye_api_key,
                    &payer,
                )
                .await;
                match handle_payments_res {
                    Ok(_) => {}
                    Err(err) => {
                        tracing::error!("{}", err.to_string());
                        // We don't panic here, this way it can try again without
                        // requiring re-deployment or re-initialization.
                    }
                }

                tokio::time::sleep(Duration::from_secs(cycle_secs)).await;
            }
        }
    }

    Ok(())
}
