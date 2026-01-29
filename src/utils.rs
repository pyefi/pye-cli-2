use std::{collections::HashSet, str::FromStr, time::Duration};

use dialoguer::Confirm;
use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    epoch_info::EpochInfo,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction;
use tracing::{error, info};

use crate::{error::PyeCliError, pye_api::LockupRewards};

pub async fn handle_epoch(
    rpc_client: &RpcClient,
    api_url: &String,
    pye_api_key: &String,
    epoch: u64,
    payer: &Keypair,
    allow_post_maturity: &HashSet<String>,
    confirm_prompt: bool,
) -> Result<(), PyeCliError> {
    // Try every 30min for 24 hours
    let lockup_rewards =
        crate::pye_api::fetch_lockup_rewards_with_retry(&api_url, &pye_api_key, epoch, 48, 1_800)
            .await?;

    // Filter out any that have matured unless the Lockup pubkey is in _allow_post_maturity_
    let now = chrono::Utc::now().timestamp();
    let lockup_rewards = lockup_rewards
        .into_iter()
        .filter(|lockup_rewards| {
            let has_not_matured =
                !lockup_rewards.maturity_handled && now <= (lockup_rewards.maturity_ts + 86_400);
            let supported_after_maturity =
                allow_post_maturity.contains(&lockup_rewards.lockup_pubkey);
            has_not_matured || supported_after_maturity
        })
        .collect();

    let transfer_info_results = determine_transfer_amounts(lockup_rewards);

    let transfer_infos = filter_results(transfer_info_results);
    info!("Transfer Infos: {:?}", transfer_infos);

    let total_transfer_amount = transfer_infos
        .iter()
        .fold(0, |agg, transfer_info| agg + transfer_info.amount);

    info!("Total SOL to be transfered: {}\n", total_transfer_amount);

    if confirm_prompt {
        let confirmed = Confirm::new()
            .with_prompt(format!(
                "Transfer {} lamports in excess rewards to all pye lockups?",
                total_transfer_amount
            ))
            .interact()?;
        if !confirmed {
            return Ok(());
        }
    }

    let transfer_instructions = generate_transfer_instructions(transfer_infos, &payer.pubkey());

    make_transfers(rpc_client, transfer_instructions, payer).await?;

    Ok(())
}

#[derive(Debug)]
pub struct TransferInfo {
    pub to: Pubkey,
    pub amount: u64,
}

/// Given lockup_rewards we calculate the expected values from each reward bucket against their
/// _base_ (aka publicly distributed) rewards.
pub fn determine_transfer_amounts(
    lockup_rewards: Vec<LockupRewards>,
) -> Vec<Result<TransferInfo, PyeCliError>> {
    lockup_rewards
        .into_iter()
        .map(|lockup_reward| {
            let inflation_delta: i64 = i64::try_from(lockup_reward.expected_inflation_rewards)?
                - i64::try_from(lockup_reward.base_inflation_rewards)?;
            let mev_delta: i64 = i64::try_from(lockup_reward.expected_mev_rewards)?
                - i64::try_from(lockup_reward.base_mev_rewards)?;
            let block_rewards_delta: i64 = i64::try_from(lockup_reward.expected_block_rewards)?
                - i64::try_from(lockup_reward.base_block_rewards)?;

            let sum = inflation_delta + mev_delta + block_rewards_delta;
            let amount = if sum.is_negative() {
                0
            } else {
                sum.unsigned_abs()
            };

            Ok(TransferInfo {
                to: Pubkey::from_str(&lockup_reward.lockup_pubkey).unwrap(),
                amount,
            })
        })
        .collect()
}

fn filter_results(transfer_infos: Vec<Result<TransferInfo, PyeCliError>>) -> Vec<TransferInfo> {
    transfer_infos
        .into_iter()
        .filter_map(|transfer_info| match transfer_info {
            Ok(transfer_info) => Some(transfer_info),
            Err(error) => {
                tracing::error!("Error generating transfer info {:?}", error);
                None
            }
        })
        .collect()
}

fn generate_transfer_instructions(
    transfer_infos: Vec<TransferInfo>,
    from_pubkey: &Pubkey,
) -> Vec<Instruction> {
    transfer_infos
        .into_iter()
        .map(|transfer_info| {
            instruction::transfer(from_pubkey, &transfer_info.to, transfer_info.amount)
        })
        .collect()
}

async fn make_transfers(
    client: &RpcClient,
    instructions: Vec<Instruction>,
    payer: &Keypair,
) -> Result<Vec<Signature>, PyeCliError> {
    let mut signatures = vec![];
    let payer_pubkey = payer.pubkey();
    for batch in instructions.chunks(50) {
        let blockhash = client.get_latest_blockhash().await?;
        let tx =
            Transaction::new_signed_with_payer(&batch, Some(&payer_pubkey), &[&payer], blockhash);

        let sig = client.send_and_confirm_transaction(&tx).await?;
        info!("TX confirmed: {}", sig);
        signatures.push(sig);
    }
    Ok(signatures)
}

pub async fn wait_for_next_epoch(
    rpc_client: &RpcClient,
    current_epoch: u64,
    cycle_secs: u64,
) -> EpochInfo {
    loop {
        tokio::time::sleep(Duration::from_secs(cycle_secs)).await;
        info!(
            "Checking for epoch boundary... current_epoch: {}",
            current_epoch
        );

        let new_epoch_info = match rpc_client.get_epoch_info().await {
            Ok(info) => info,
            Err(e) => {
                error!("Error getting epoch info: {:?}", e);
                continue;
            }
        };

        if new_epoch_info.epoch > current_epoch {
            info!(
                "New epoch detected: {} -> {}",
                current_epoch, new_epoch_info.epoch
            );
            return new_epoch_info;
        }
    }
}
