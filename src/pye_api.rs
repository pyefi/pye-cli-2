use crate::utils::PaymentInfo;
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
    /// Can be null in DB (BOOLEAN DEFAULT FALSE without NOT NULL)
    #[serde(default)]
    pub is_jito_claim: Option<bool>,
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

/// Response from get_cli_alert_info edge function.
#[derive(Debug, Deserialize)]
pub struct CliAlertInfo {
    pub cli_low_balance_alert_last_epoch: Option<u64>,
    #[serde(rename = "avg_expected_amount_last_3_epochs")]
    pub avg_expected_amount_last_3_epochs: String,
}

pub async fn get_cli_alert_info(url: &str, api_key: &str) -> Result<CliAlertInfo, PyeCliError> {
    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let res = client
        .get(format!("{}/functions/v1/get_cli_alert_info", url))
        .header("x-api-key", api_key)
        .header("Content-Type", "application/json")
        .send()
        .await?;

    let status = res.status().as_u16();
    let body = res.text().await?;
    if status >= 300 {
        Err(PyeCliError::PyeApiError(status, body))
    } else {
        serde_json::from_str(&body).map_err(Into::into)
    }
}

pub async fn fetch_bond_payments_v2(
    url: &str,
    api_key: &str,
) -> Result<Vec<BondPaymentsV2>, PyeCliError> {
    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

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
    payment_infos: &[PaymentInfo],
    signature: &str,
) -> Result<(), PyeCliError> {
    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let payload = serde_json::json!({
        "payment_infos": payment_infos,
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
            payment_infos.len()
        );
        Ok(())
    }
}

/// Sends a low balance alert email via the backend (Resend). Call only when balance < threshold and rate limit allows.
pub async fn send_cli_low_balance_alert(
    url: &str,
    api_key: &str,
    recipient_email: &str,
    current_epoch: u64,
    balance_lamports: u64,
) -> Result<(), PyeCliError> {
    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let payload = serde_json::json!({
        "recipient_email": recipient_email,
        "current_epoch": current_epoch,
        "balance_lamports": balance_lamports.to_string()
    });

    let res = client
        .post(format!("{}/functions/v1/send_cli_low_balance_alert", url))
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
        Ok(())
    }
}

/// Sends a CLI heartbeat to the backend (payer pubkey, balance, version).
/// Used for telemetry; non-fatal if the request fails.
/// When `alert_sent_for_epoch` is `Some`, the backend updates `cli_low_balance_alert_last_epoch` (call only after sending an alert).
pub async fn report_cli_heartbeat(
    url: &str,
    api_key: &str,
    payer_pubkey: &str,
    payer_balance_lamports: u64,
    version: &str,
    alert_sent_for_epoch: Option<u64>,
) -> Result<(), PyeCliError> {
    let client = Client::builder()
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let mut payload = serde_json::json!({
        "payer_pubkey": payer_pubkey,
        "payer_balance_lamports": payer_balance_lamports.to_string(),
        "version": version
    });
    if let Some(epoch) = alert_sent_for_epoch {
        payload["current_epoch"] = serde_json::json!(epoch);
    }

    let res = client
        .post(format!("{}/functions/v1/cli_heartbeat", url))
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
        Ok(())
    }
}
