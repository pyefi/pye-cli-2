use std::time::Duration;

use clap::{Parser, Subcommand};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::read_keypair_file;

use crate::{error::PyeCliError, utils::wait_for_next_epoch};

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
    #[arg(long, env, default_value = "https://gwtgzlzfnztqhiulhgtm.supabase.co")]
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

    HandleEpoch {
        #[command(flatten)]
        args: CommonHandlerArgs,
        #[arg(long, env)]
        epoch: u64,
    },
}

#[tokio::main]
async fn main() -> Result<(), PyeCliError> {
    println!("Hello, world!");
    let cli = Cli::parse();
    match cli.command {
        Commands::ValidatorLockupManager { args, cycle_secs } => {
            let payer = read_keypair_file(&args.payer)
                .map_err(|err| PyeCliError::ReadKeypairError(err.to_string()))?;

            let rpc_client = RpcClient::new(args.rpc_url.clone());
            let mut current_epoch_info = rpc_client.get_epoch_info().await?;
            loop {
                // Wait for next epoch
                current_epoch_info =
                    wait_for_next_epoch(&rpc_client, current_epoch_info.epoch, cycle_secs).await;

                // We wait 12 hours before handling the epoch because it takes time for the Pye
                // backend to obtain and aggregate the relevant epoch rewards data.
                tokio::time::sleep(Duration::from_secs(43_200)).await;

                crate::utils::handle_epoch(
                    &rpc_client,
                    &args.api_url,
                    &args.pye_api_key,
                    current_epoch_info.epoch,
                    &payer,
                    false,
                )
                .await?;
            }
        }
        Commands::HandleEpoch { args, epoch } => {
            let payer = read_keypair_file(&args.payer)
                .map_err(|err| PyeCliError::ReadKeypairError(err.to_string()))?;

            let rpc_client = RpcClient::new(args.rpc_url);

            crate::utils::handle_epoch(
                &rpc_client,
                &args.api_url,
                &args.pye_api_key,
                epoch,
                &payer,
                true,
            )
            .await?;
        }
    }

    Ok(())
}
