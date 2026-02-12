use log::info;
use reqwest::Client;
use serde::Deserialize;
use serde_json;
use serde_with::{DisplayFromStr, serde_as};

use crate::error::PyeCliError;

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct BondPaymentsV2 {
    pub id: String,
    pub bond_pubkey: String,
    #[serde_as(as = "DisplayFromStr")]
    pub amount: u64,
    pub fee_payer: Option<String>,
    pub is_jito_claim: bool,
    #[serde_as(as = "DisplayFromStr")]
    pub epoch: u64,
    pub finalized: bool,
    pub signature: Option<String>,
    pub finalization_attempts: u16,
    #[serde_as(as = "DisplayFromStr")]
    pub expected_amount: u64,
    pub organization_id: u64,
    pub validator_vote_account: String,
}

pub async fn fetch_bond_payments_v2(
    url: &str,
    api_key: &str,
) -> Result<Vec<BondPaymentsV2>, PyeCliError> {
    let client = Client::new();

    let res = client
        .get(format!("{}/functions/v1/bond_payments_v2", url))
        .header("x-api-key", api_key)
        .header("Content-Type", "application/json")
        .send()
        .await?;
    let status = res.status().as_u16();
    let body = res.text().await?;
    if status >= 300 {
        Err(PyeCliError::PyeApiError(status, body))
    } else {
        let res = serde_json::from_str(&body)?;

        Ok(res)
    }
}

pub async fn update_bond_payments_signatures(
    url: &str,
    api_key: &str,
    payment_ids: &[String],
    signature: &str,
) -> Result<(), PyeCliError> {
    let client = Client::new();

    let payload = serde_json::json!({
        "payment_ids": payment_ids,
        "signature": signature
    });

    let res = client
        .post(format!(
            "{}/functions/v1/update_bond_payment_signatures",
            url
        ))
        .header("x-api-key", api_key)
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await?;

    let status = res.status().as_u16();
    let body = res.text().await?;

    if status >= 300 {
        Err(PyeCliError::PyeApiError(status, body))
    } else {
        info!(
            "Successfully updated signatures for {} payments",
            payment_ids.len()
        );
        Ok(())
    }
}
