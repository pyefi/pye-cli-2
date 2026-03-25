# pye-cli

A CLI tool for processing bond payments on Solana through the Pye backend. Automatically fetches pending payments, creates transfer instructions, and submits transactions while maintaining payment tracking via instruction indices.

## Requirements

- Rust 2024 edition
- Solana keypair file (payer with SOL for transaction fees)
- Pye API key for backend authentication

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
RUST_LOG=info
```

## Usage

### Daemon Mode

```bash
pye-cli validator-lockup-manager \
  --payer /path/to/keypair.json \
  --pye-api-key your-api-key
```

Continuously polls the Pye backend for pending bond payments and processes them automatically. The wait between cycles is fixed at **5 minutes** (configured in code, not via flags or env).

**How it works:**
See [CLI_COMMANDS.md](./CLI_COMMANDS.md) for detailed documentation of the process.

## Payment Processing

The CLI processes payments where:

```
transfer_amount = expected_amount - amount
```

Only positive transfer amounts are processed. The backend tracks:
- Transaction signatures
- Instruction indices (position in transaction)
- Payment finalization status

## Architecture

```
┌─────────────┐
│   Pye CLI   │
└──────┬──────┘
       │
       ├─→ GET /functions/v1/bond_payments_v2
       │   (Fetch pending payments)
       │
       ├─→ POST /functions/v1/update_bond_payment_signatures
       │   (Register signature + instruction_index)
       │
       └─→ Solana RPC
           (Submit transaction)
```

The instruction index is crucial for the backend to verify which payment corresponds to which instruction when parsing the on-chain transaction.

## Commands Reference

See [CLI_COMMANDS.md](./CLI_COMMANDS.md) for detailed command documentation.

## License

[BUSL-1.1](/LICENSE.md)
