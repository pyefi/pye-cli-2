use std::{collections::HashSet, time::Duration};

use clap::{Parser, Subcommand};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::read_keypair_file;
use tracing::{Level, info};
use tracing_subscriber::EnvFilter;

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
    /// List of Pye Lockup pubkeys that should continue receiving payments after maturity.
    #[arg(long, env, value_delimiter = ',')]
    allow_post_maturity: Vec<String>,
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
    info!("Hello, world!");
    let cli = Cli::parse();
    match cli.command {
        Commands::ValidatorLockupManager { args, cycle_secs } => {
            let payer = read_keypair_file(&args.payer)
                .map_err(|err| PyeCliError::ReadKeypairError(err.to_string()))?;

            let rpc_client = RpcClient::new(args.rpc_url.clone());
            let mut current_epoch_info = rpc_client.get_epoch_info().await?;
            let allow_post_maturity: HashSet<String> =
                args.allow_post_maturity.into_iter().map(|x| x).collect();
            loop {
                // Wait for next epoch
                current_epoch_info =
                    wait_for_next_epoch(&rpc_client, current_epoch_info.epoch, cycle_secs).await;

                // We wait 12 hours before handling the epoch because it takes time for the Pye
                // backend to obtain and aggregate the relevant epoch rewards data.
                tokio::time::sleep(Duration::from_secs(43_200)).await;

                let target_epoch = current_epoch_info.epoch - 1;

                let handle_epoch_res = crate::utils::handle_epoch(
                    &rpc_client,
                    &args.api_url,
                    &args.pye_api_key,
                    target_epoch,
                    &payer,
                    &allow_post_maturity,
                    false,
                )
                .await;
                match handle_epoch_res {
                    Ok(_) => {},
                    Err(err) => {
                        tracing::error!("{}", err.to_string());
                        // We don't panic here, this way it can try again next epoch without 
                        // requiring re-deployment or re-initialization.
                    },
                }
            }
        }
        Commands::HandleEpoch { args, epoch } => {
            let payer = read_keypair_file(&args.payer)
                .map_err(|err| PyeCliError::ReadKeypairError(err.to_string()))?;

            let rpc_client = RpcClient::new(args.rpc_url);

            let allow_post_maturity: HashSet<String> =
                args.allow_post_maturity.into_iter().map(|x| x).collect();
            crate::utils::handle_epoch(
                &rpc_client,
                &args.api_url,
                &args.pye_api_key,
                epoch,
                &payer,
                &allow_post_maturity,
                true,
            )
            .await?;
        }
    }

    Ok(())
}
