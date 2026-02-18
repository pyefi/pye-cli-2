use std::str::FromStr;

use solana_instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction;
use tracing::info;

use crate::error::PyeCliError;

struct TransferInstuctionWithPaymentId {
    pub instruction: Instruction,
    pub payment_id: String,
}

#[derive(serde::Serialize)]
pub struct PaymentInfo {
    pub payment_id: String,
    pub instruction_index: u64,
}

pub async fn handle_payments_to_be_sent(
    rpc_client: &RpcClient,
    api_url: &String,
    pye_api_key: &String,
    payer: &Keypair,
) -> Result<(), PyeCliError> {
    let payments = crate::pye_api::fetch_bond_payments_v2(&api_url, &pye_api_key).await?;
    info!("handle_payments_to_be_sent: Payments: {:?}", payments.len());

    // step 1: create instructions
    let mut transfer_instructions_with_payment_ids = vec![];

    for payment in payments {
        // @todo check what if amount is null?
        let payment_amount = payment.expected_amount - payment.amount;

        if payment_amount <= 0 {
            continue;
        }

        let transfer_instruction = instruction::transfer(
            &payer.pubkey(),
            &Pubkey::from_str(&payment.bond_pubkey).unwrap(),
            payment_amount,
        );

        transfer_instructions_with_payment_ids.push(TransferInstuctionWithPaymentId {
            instruction: transfer_instruction,
            payment_id: payment.id,
        });
    }

    // step 2: make transfers
    let payer_pubkey = payer.pubkey();
    for batch in transfer_instructions_with_payment_ids.chunks(50) {
        // WARNING: in the below transaction, instruction order matters.
        // please check the readme for more details.
        let instructions: Vec<_> = batch.iter().map(|x| x.instruction.clone()).collect();
        let blockhash = rpc_client.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &instructions,
            Some(&payer_pubkey),
            &[&payer],
            blockhash,
        );

        // Get the signature of the transaction without sending it
        let signature = tx.signatures[0];

        // Collect payment IDs for this batch
        let payment_ids: Vec<String> = batch.iter().map(|x| x.payment_id.clone()).collect();

        let mut payment_infos: Vec<PaymentInfo> = Vec::new();
        for (i, payment_id) in payment_ids.iter().enumerate() {
            payment_infos.push(PaymentInfo {
                payment_id: payment_id.clone(),
                instruction_index: i as u64,
            });
        }

        // Save the signature in the backend DB along with the payment ids
        crate::pye_api::update_bond_payments_signatures(
            api_url,
            pye_api_key,
            &payment_infos,
            &signature.to_string(),
        )
        .await?;

        // Send the transaction
        let sig = rpc_client.send_transaction(&tx).await?;

        info!("Transaction sent successfully: {}", sig);
    }

    Ok(())
}

#[derive(Debug)]
pub struct TransferInfo {
    pub to: Pubkey,
    pub amount: u64,
}
