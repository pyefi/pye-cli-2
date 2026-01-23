use std::time::Duration;

use log::info;
use reqwest::Client;
use serde::Deserialize;
use serde_with::{DisplayFromStr, serde_as};

use crate::error::PyeCliError;

#[serde_as]
#[derive(Deserialize)]
pub struct LockupRewards {
    pub validator_vote_account: String,
    pub lockup_pubkey: String,
    #[serde_as(as = "DisplayFromStr")]
    pub epoch: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub base_inflation_rewards: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub expected_inflation_rewards: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub base_mev_rewards: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub expected_mev_rewards: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub base_block_rewards: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub expected_block_rewards: u64,
    #[serde_as(as = "DisplayFromStr")]
    pub maturity_ts: i64,
    pub stake_account: String,
    pub transient_stake_account: String,
    pub inflation_bps: u16,
    pub mev_tips_bps: u16,
    pub block_rewards_bps: u16,
    pub issuer: String,
    pub maturity_handled: bool,
}

pub async fn fetch_lockup_rewards(
    url: &str,
    api_key: &str,
    epoch: u64,
) -> Result<Vec<LockupRewards>, PyeCliError> {
    let client = Client::new();

    let res = client
        .get(format!(
            "{}/functions/v1/lockup_rewards?epoch={}",
            url, epoch
        ))
        .header("x-api-key", api_key)
        .header("Content-Type", "application/json")
        .send()
        .await?;
    let body = res.text().await?;
    let res = serde_json::from_str(&body)?;

    Ok(res)
}

pub async fn fetch_lockup_rewards_with_retry(
    url: &str,
    api_key: &str,
    epoch: u64,
    max_attempts: u16,
    wait_secs: u64,
) -> Result<Vec<LockupRewards>, PyeCliError> {
    let mut atempts: u16 = 0;
    loop {
        let res = fetch_lockup_rewards(url, api_key, epoch).await?;
        if res.is_empty() {
            atempts += 1;
            if atempts >= max_attempts {
                return Err(PyeCliError::FetchRewardsMaxAttempts);
            }
            info!(
                "No LockupRewards found for Organization's validators. Attempt {}. Retrying...",
                atempts
            );
            tokio::time::sleep(Duration::from_secs(wait_secs)).await;
            continue;
        }
        return Ok(res);
    }
}
