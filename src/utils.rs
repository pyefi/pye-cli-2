use std::str::FromStr;

use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction;

use crate::{error::PyeCliError, pye_api::LockupRewards};


pub async fn handle_epoch(api_url: String, pye_api_key: String, epoch: u64, ) -> Result<(), PyeCliError> {
  let lockup_rewards = crate::pye_api::fetch_lockup_rewards(&api_url, &pye_api_key, epoch).await?;

  let transfer_infos = determine_transfer_amounts(lockup_rewards);
  Ok(())
}

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

fn generate_transfer_instructions(
    transfer_infos: Vec<Result<TransferInfo, PyeCliError>>,
    from_pubkey: &Pubkey,
) -> Vec<Instruction> {
    transfer_infos
        .into_iter()
        .filter_map(|transfer_info| match transfer_info {
            Ok(transfer_info) => Some(instruction::transfer(
                from_pubkey,
                &transfer_info.to,
                transfer_info.amount,
            )),
            Err(error) => {
                tracing::error!("Error generating transfer info {:?}", error);
                None
            }
        })
        .collect()
}

pub async fn make_transfers(
    client: &RpcClient,
    instructions: Vec<Instruction>,
    payer: Keypair,
) -> Result<Vec<Signature>, PyeCliError> {
    let mut signatures = vec![];
    let payer_pubkey = payer.pubkey();
    for batch in instructions.chunks(50) {
        let blockhash = client.get_latest_blockhash().await?;
        let tx =
            Transaction::new_signed_with_payer(&batch, Some(&payer_pubkey), &[&payer], blockhash);

        let sig = client.send_and_confirm_transaction(&tx).await?;
        signatures.push(sig);
    }
    Ok(signatures)
}
