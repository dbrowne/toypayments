# Toy Payments Engine

A simple transaction processing engine that reads financial transactions from a CSV file, processes deposits, withdrawals, disputes, resolutions, and chargebacks, then outputs the final state of all client accounts.
### Build and tested with Rust 1.92.0

## Building and Running

```bash
# Build
cargo build --release

# Run
cargo run -- transactions.csv > accounts.csv
or 
cargo run --release -- transactions.csv > accounts.csv

# Run tests
cargo test
```

## Input Format

CSV file with columns: `type`, `client`, `tx`, `amount`

```csv
type,client,tx,amount
deposit,1,1,100.0
deposit,2,2,50.0
withdrawal,1,3,25.5
dispute,1,1,
resolve,1,1,
chargeback,2,2,
```

- `type`: Transaction type (deposit, withdrawal, dispute, resolve, chargeback)
- `client`: Client ID (u16)
- `tx`: Transaction ID (u32, globally unique)
- `amount`: Decimal with up to 4 decimal places (required for deposit/withdrawal, empty for others)

Whitespace around values is handled automatically.

## Output Format

CSV to stdout with columns: `client`, `available`, `held`, `total`, `locked`

```csv
client,available,held,total,locked
1,74.5000,0.0000,74.5000,false
2,0.0000,0.0000,0.0000,true
```

Output is sorted by client ID.

## Transaction Types

| Type | Description |
|------|-------------|
| **deposit** | Credits funds to client's available balance |
| **withdrawal** | Debits funds from client's available balance (fails if insufficient funds) |
| **dispute** | Moves funds from available to held for a referenced transaction |
| **resolve** | Releases disputed funds back to available |
| **chargeback** | Removes held funds and locks the account |

## Design Decisions

### Error Handling

- Invalid transactions (parse errors, business logic failures) are logged to `errors.log` if the file can be created, otherwise silently ignored
- The engine continues processing subsequent transactions after errors
- This follows the spec's guidance to "ignore" invalid disputes/resolves/chargebacks

### Disputes

- Only deposits can be disputed see  comment in Ambiguities in the Spec
- A dispute requires sufficient available funds 
- The same transaction can be disputed again after being resolved
- Client ID must match between the dispute and the original transaction

### Locked Accounts

- An account is locked after a chargeback
- Locked accounts reject new deposits and withdrawals
- Disputes, resolves, and chargebacks can still be processed on locked accounts

### Precision

- Uses `rust_decimal` for arbitrary-precision decimal arithmetic
- Avoids floating-point rounding errors
- Output formatted to 4 decimal places
- Overflow is not handled due to the scope of this project and the fact that rust_decimal::Decimal can hold ~79 octillion. We should be  good for this problem

### Memory Usage

- Transactions are streamed from the CSV (not loaded entirely into memory)
- However, all processed deposit/withdrawal transactions are stored in a HashMap for potential disputes
- For very large files (billions of transactions), this may exceed available memory
- Successfully tested with a 60GB transaction file containing 1.7B transactions. 
- git checkout 332a5ad; cargo run --release --bin  generate-transactions ; cargo run --release generated_transactions.csv >accounts.csv 

## Assumptions

1. Transaction IDs are globally unique across all clients
2. Transactions occur chronologically as they appear in the file
3. A client has a single asset account
4. Negative amounts are rejected
5. Zero-amount transactions are allowed (no-op)
6. Duplicate transaction IDs are rejected
7. Written for a non concurrent implementation see my comments in src/engine.rs


## Ambiguities in the spec and my interpretation
1. Dispute with insufficient available funds - If a client deposits 100, withdraws 80, then disputes the original deposit, we fail with InsufficientFunds.
2. Disputing withdrawals - Rejected with CannotDisputeWithdrawal. The spec only shows deposit disputes in examples but doesn't explicitly forbid withdrawal disputes so...
3. Re-disputing after resolve - I'm allowing disputing a transaction again after it's been resolved since it could happen in real life unless explicltly stated.
4. The spec says I can ignore errors. I prefer to log  them if possible since financial transactions require logging.
5. Alowing other transactions to be disputed since locking an account only prevents deposits or withdrawals.


## Testing

The project includes:

- **Unit tests** for account operations, engine logic, and decimal formatting
- **Integration tests** that run the binary against various CSV inputs

Run all tests:
```bash
cargo test
```

### Test Coverage

- Basic deposits and withdrawals
- Insufficient funds handling
- Dispute/resolve/chargeback flows
- Locked account behavior
- Client mismatch detection
- Duplicate transaction rejection
- Decimal precision
- Whitespace handling
- Empty files
- Multiple clients with ordering

## Project Structure

```
src/
  main.rs                     # CLI and CSV I/O
  engine.rs                   # Transaction processing logic
  account.rs                  # Account state and operations
  transaction.rs              # Transaction types and parsing
tests/
  integration.rs              # End-to-end binary tests
 bin/
   generate_transactions.rs   # AI generated csv file generator for testing REMOVED CHECKOUT PREV. VERSION
```

## Dependencies

- `csv` - CSV parsing
- `serde` - Serialization/deserialization
- `rust_decimal` - Precise decimal arithmetic
- `anyhow` / `thiserror` - Error handling
- `tracing` - Logging (optional, via RUST_LOG env var)

## Additional code (removed due to auto testing issues check out prev. ver to see)
- bin/generate_transactions A 100% AI created csv generator that reads generator_params.toml for config


# AI DECLARATION
## The rust rover Claude AI plugin was used to 
- Generate test cases
- Convert errors to use thiserror and anyhow
- Create a csv generation tool bin/generate_transactions
- Create the initial version of this README.md 




## Human written code:
- The datatypes and their implementation 
- The base algorithms to process the code