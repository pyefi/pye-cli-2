# pye-cli-2 Commands

A Solana-based CLI for managing validator lockup rewards distribution.

## Global Options

All commands accept these options:

| Option | Env Var | Default | Description |
|--------|---------|---------|-------------|
| `-r, --rpc-url` | `RPC_URL` | `https://api.mainnet-beta.solana.com` | Solana RPC endpoint |
| `-p, --payer` | `PAYER` | Required | Path to payer keypair file |
| `--pye-api-key` | `PYE_API_KEY` | Required | Pye API authentication key |
| `--api-url` | `API_URL` | `https://gwtgzlzfnztqhiulhgtm.supabase.co` | Pye backend API URL |

## Commands

### `validator-lockup-manager`

Continuously monitors and distributes excess rewards for all pye lockup accounts owned by a validator. Runs as a daemon that waits for epoch changes.

```bash
pye-cli-2 validator-lockup-manager [OPTIONS]
```

**Options:**

| Option | Env Var | Default | Description |
|--------|---------|---------|-------------|
| `--cycle-secs` | `CYCLE_SECS` | `60` | Seconds between epoch change checks |

**Behavior:**
1. Waits for the next epoch to begin
2. Waits 12 hours for Pye backend to aggregate rewards data
3. Fetches lockup rewards from Pye API
4. Calculates and transfers excess rewards automatically (no confirmation prompt)

---

### `handle-epoch`

Manually processes rewards for a specific epoch. Useful for one-time execution or testing.

```bash
pye-cli-2 handle-epoch --epoch <EPOCH> [OPTIONS]
```

**Options:**

| Option | Env Var | Default | Description |
|--------|---------|---------|-------------|
| `--epoch` | `EPOCH` | Required | The epoch number to process |

**Behavior:**
1. Fetches lockup rewards from Pye API (retries up to 24 times with 1-hour intervals)
2. Calculates excess rewards (expected - base) for inflation, MEV, and block rewards
3. Prompts for user confirmation before transferring
4. Batches transfers (50 per transaction) and submits to the blockchain

## Reward Calculation

Excess rewards are calculated as:

```
transfer_amount = (expected_inflation - base_inflation)
                + (expected_mev - base_mev)
                + (expected_block - base_block)
```

If the sum is negative, no transfer occurs.
