# pye-cli Commands

A Solana-based CLI for managing bond payment distributions through the Pye backend.

## Global Options

All commands accept these options:

| Option | Env Var | Default | Description |
|--------|---------|---------|-------------|
| `-r, --rpc-url` | `RPC_URL` | `https://api.mainnet-beta.solana.com` | Solana RPC endpoint |
| `-p, --payer` | `PAYER` | Required | Path to payer keypair file |
| `--pye-api-key` | `PYE_API_KEY` | Required | Pye API authentication key |
| `--api-url` | `API_URL` | `https://ABCDEFG.supabase.co` | Pye backend API URL |

## Commands

### `validator-lockup-manager`

Continuously monitors and processes bond payments. Runs as a daemon that polls the Pye backend for pending payments.

```bash
pye-cli validator-lockup-manager [OPTIONS]
```

**Cycle interval:** The daemon waits **5 minutes** between iterations. This is hardcoded in the binary (not configurable via CLI or environment).

**Behavior:**

1. **Fetch Pending Payments**: Queries the Pye backend API for `bond_payments_v2` records that need to be processed
2. **Create Transfer Instructions**: For each payment, creates a SOL transfer instruction from the payer to the bond account
3. **Batch Transactions**: Groups transfers into batches of 50 instructions per transaction
4. **Pre-register Signatures**: Before sending each transaction, calls the backend API with:
   - Transaction signature
   - Array of `payment_infos` containing:
     - `payment_id`: The database ID of the payment
     - `instruction_index`: The position of this payment's instruction in the transaction (0-based)
5. **Submit Transactions**: Sends the signed transaction to the Solana network
6. **Backend checking tx**: The backend will attempt to confirm the transaction every minute for 15 minutes. If the TX is still not confirmed, then the payment will be re-tried.
7. **Continuous Loop**: Waits 5 minutes and repeats

**Payment Calculation:**

For each payment, the transfer amount is calculated as:

```
transfer_amount = expected_amount - amount
```

If `transfer_amount <= 0`, the payment is skipped.

**Important Notes:**

- The CLI automatically transfers funds without user confirmation
- Transaction instruction order is critical - the `instruction_index` must match the actual position in the transaction
- The backend uses the signature and instruction indices to track and verify payments on-chain
- Failed transactions will be retried in subsequent cycles

## API Integration

The CLI interacts with two Pye backend endpoints:

### `GET /functions/v1/bond_payments_v2`

Fetches pending payments to process.

**Response:** Array of `BondPaymentsV2` objects containing:
- `id`: Payment ID
- `bond_pubkey`: Destination bond account
- `amount`: Base amount already paid
- `expected_amount`: Total amount that should be paid
- `epoch`: The epoch this payment is for
- Other metadata fields

### `POST /functions/v1/update_bond_payment_signatures`

Updates payment records with transaction details.

**Request Body:**
```json
{
  "signature": "transaction_signature_string",
  "payment_infos": [
    {
      "payment_id": "uuid",
      "instruction_index": 0
    }
  ]
}
```

**Response:** Confirmation of updated records
