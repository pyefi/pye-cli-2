use std::time::Duration;

use clap::{Parser, Subcommand};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::signature::read_keypair_file;
use solana_sdk::signer::Signer;
use tracing::Level;
use tracing_subscriber::EnvFilter;

use crate::error::PyeCliError;

pub mod error;
pub mod pye_api;
pub mod utils;

/// CLI version sent in heartbeat telemetry.
/// Use Semantic Versioning (SemVer): MAJOR.MINOR.PATCH
/// - MAJOR: breaking changes / incompatible API
/// - MINOR: new features, backward compatible
/// - PATCH: bug fixes only, backward compatible
const CLI_HEARTBEAT_VERSION: &str = "2.0.1";

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
    /// Email address for low balance alerts (optional; if set, alerts are sent when balance is below threshold)
    #[arg(long, env)]
    cli_alert_email: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Will run the excess rewards stuff for all pye_accounts owned by a validator
    ValidatorLockupManager {
        #[command(flatten)]
        args: CommonHandlerArgs,
        /// The wait time (in secs) between epoch change checks
        #[arg(long, env, default_value = "300")]
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
                // 1. Process payments first so that when we check balance (and maybe alert), the amount reflects post-payout state
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

                // 2. Check balance and maybe send low-balance alert (only past half-epoch, when payouts are done)
                // CLI heartbeat: fetch alert info, maybe send low-balance alert, then report telemetry
                let balance_lamports = rpc_client.get_balance(&payer.pubkey()).await.unwrap_or(0);

                let epoch_info = rpc_client.get_epoch_info().await.ok();
                let current_epoch = epoch_info.as_ref().map(|info| info.epoch);
                // Only consider sending low-balance alert past half-epoch, when payouts have almost certainly been processed
                let past_half_epoch = epoch_info
                    .as_ref()
                    .map(|info| info.slot_index * 2 >= info.slots_in_epoch)
                    .unwrap_or(false);

                let alert_info =
                    crate::pye_api::get_cli_alert_info(&args.api_url, &args.pye_api_key).await;

                let (_should_alert, alert_sent_for_epoch) = match (current_epoch, alert_info) {
                    (Some(epoch), Ok(info)) => {
                        let avg_str = info.avg_expected_amount_last_3_epochs.trim();
                        // API returns decimal string (e.g. "197164398.00000000"); u64 parse fails on the dot, so take integer part
                        let avg_lamports: u64 = avg_str
                            .split('.')
                            .next()
                            .unwrap_or(avg_str)
                            .parse()
                            .unwrap_or(0);
                        let threshold = avg_lamports.saturating_mul(3);
                        let below_threshold = balance_lamports < threshold;
                        let already_alerted = info
                            .cli_low_balance_alert_last_epoch
                            .map(|last| epoch <= last)
                            .unwrap_or(false);
                        let should = below_threshold
                            && !already_alerted
                            && avg_lamports > 0
                            && past_half_epoch;

                        let sent_alert = if should {
                            args.cli_alert_email
                                .as_ref()
                                .map(|e| !e.trim().is_empty())
                                .unwrap_or(false)
                                && {
                                    if let Err(e) = crate::pye_api::send_cli_low_balance_alert(
                                        &args.api_url,
                                        &args.pye_api_key,
                                        args.cli_alert_email.as_deref().unwrap_or(""),
                                        epoch,
                                        balance_lamports,
                                    )
                                    .await
                                    {
                                        tracing::warn!(
                                            "Failed to send low balance alert email: {}",
                                            e
                                        );
                                        false
                                    } else {
                                        true
                                    }
                                }
                        } else {
                            false
                        };
                        (should, if sent_alert { Some(epoch) } else { None })
                    }
                    _ => (false, None),
                };

                if let Err(e) = crate::pye_api::report_cli_heartbeat(
                    &args.api_url,
                    &args.pye_api_key,
                    &payer.pubkey().to_string(),
                    balance_lamports,
                    CLI_HEARTBEAT_VERSION,
                    alert_sent_for_epoch,
                )
                .await
                {
                    tracing::warn!("CLI heartbeat failed: {}", e);
                }

                tokio::time::sleep(Duration::from_secs(cycle_secs)).await;
            }
        }
    }
}
