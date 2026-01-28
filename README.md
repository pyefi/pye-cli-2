# pye-cli

A CLI tool for managing Pye Lockup excess rewards distribution on Solana. Automates calculation and transfer of excess validator rewards (inflation, MEV tips, and block rewards) to lockup accounts.

## Requirements

- Rust 2024 edition
- Solana keypair file (payer with SOL for fees)
- Pye API key

## Installation

```bash
cargo build --release
```

## Configuration

Configure via CLI arguments or environment variables:

```bash
# Required
PAYER=/path/to/keypair.json
PYE_API_KEY=your-api-key

# Optional
RPC_URL=https://api.mainnet-beta.solana.com
API_URL=https://ABCDEFG.supabase.co
CYCLE_SECS=60
ALLOW_POST_MATURITY=pubkey1,pubkey2
RUST_LOG=info
```

## Usage

### Daemon Mode

```bash
pye-cli validator-lockup-manager \
  --payer /path/to/keypair.json \
  --pye-api-key your-api-key
```

Monitors for new epochs and automatically distributes excess rewards.

### Manual Epoch Processing

```bash
pye-cli handle-epoch \
  --epoch 500 \
  --payer /path/to/keypair.json \
  --pye-api-key your-api-key
```

Prompts for confirmation before transfers.

## Commands Reference

See [CLI_COMMANDS.md](./CLI_COMMANDS.md) for detailed command documentation.

## License

[BUSL-1.1](/LICENSE.md)
