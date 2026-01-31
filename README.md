# light-instruction-decoder

Solana instruction decoding library for testing and transaction logging. Decodes raw instruction bytes into human-readable fields, account names, and instruction names.

Supports Anchor programs (8-byte discriminators), native programs (1-byte, 4-byte), and ships with built-in decoders for System Program, SPL Token, Token 2022, Compute Budget, and Light Protocol programs.

```sh
RUST_BACKTRACE=1 cargo test -p my-tests -- --nocapture
```

## Example output

```
┌──────────────────────────────────────────────────────────── Transaction #1 ─────────────────────────────────────────────────────────────┐
│ Transaction: 4DySUV8MPozC8yUfFqX9J7r5azJz7MRvhebUjqkQTgmcHHJiFhQptpABSkBe1emRF5odQHYCKU5wrwKMh4bkZrGB | Slot: 0 | Status: Success
│ Fee: 0.000010 SOL | Compute Used: 4413/1400000 CU
│
│ Instructions (1):
│
│ ├─ #1.1 Counter111111111111111111111111111111111111 (Counter) - Initialize
│ │  Accounts (3):
│ │  +----+----------------------------------------------+-----------------+----------------+-------+----------+----------------+------------+
│ │  | #  | Account                                      | Type            | Name           | Owner | Data Len | Lamports       | Change     |
│ │  +----+----------------------------------------------+-----------------+----------------+-------+----------+----------------+------------+
│ │  | #1 | 7v54NWdBtkjuAFJrLGsS2SXnuk8nKam81mZJeeYxVFi9 | signer+writable | counter        | 11111 | 0        | 0              | +1,224,960 |
│ │  +----+----------------------------------------------+-----------------+----------------+-------+----------+----------------+------------+
│ │  | #2 | 5Z6Ay5NEcbg3xhopc522sBCRXQujkTiuDRnHGfQdcnSf | signer+writable | authority      | 11111 | 0        | 10,000,000,000 | -1,234,960 |
│ │  +----+----------------------------------------------+-----------------+----------------+-------+----------+----------------+------------+
│ │  | #3 | 11111111111111111111111111111111             | readonly        | system_program | Nativ | 14       | 1              | 0          |
│ │  +----+----------------------------------------------+-----------------+----------------+-------+----------+----------------+------------+
│ │  └─ #1 11111111111111111111111111111111 (System Program) - CreateAccount
│ │  │    lamports: 1224960
│ │  │    space: 48
│ │  │  Accounts (2):
│ │  │  +----+----------------------------------------------+-----------------+-----------------+
│ │  │  | #  | Account                                      | Type            | Name            |
│ │  │  +----+----------------------------------------------+-----------------+-----------------+
│ │  │  | #1 | 5Z6Ay5NEcbg3xhopc522sBCRXQujkTiuDRnHGfQdcnSf | signer+writable | funding_account |
│ │  │  +----+----------------------------------------------+-----------------+-----------------+
│ │  │  | #2 | 7v54NWdBtkjuAFJrLGsS2SXnuk8nKam81mZJeeYxVFi9 | signer+writable | new_account     |
│ │  │  +----+----------------------------------------------+-----------------+-----------------+
│
│ Program Logs:
│
│ Program Counter111111111111111111111111111111111111 invoke [1]
│ Instruction: Initialize
│ Program 11111111111111111111111111111111 invoke [2]
│ Program 11111111111111111111111111111111 success
│ Program Counter111111111111111111111111111111111111 consumed 4413 of 200000 compute units
│ Program Counter111111111111111111111111111111111111 success
└──────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

## Usage

### Anchor programs

Annotate your program module with `#[instruction_decoder]`:

```rust
use anchor_lang::prelude::*;
use light_instruction_decoder_derive::instruction_decoder;

declare_id!("Counter111111111111111111111111111111111111");

#[instruction_decoder]
#[program]
pub mod counter {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> { /* ... */ }
    pub fn set(ctx: Context<Set>, value: u64) -> Result<()> { /* ... */ }
}
```

This generates a `CounterInstructionDecoder` that automatically extracts instruction names, account names from `Accounts` structs, and parameter types.

### Native programs

Use the derive macro on an enum:

```rust
use light_instruction_decoder_derive::InstructionDecoder;

#[derive(InstructionDecoder)]
#[instruction_decoder(
    program_id = "TokenkegQfeZyiNwAJbPVwwQQfKP5nS6Unj84UMP",
    discriminator_size = 1
)]
pub enum TokenInstruction {
    #[instruction_decoder(account_names = ["mint", "account", "owner"])]
    MintTo { amount: u64 },

    #[instruction_decoder(account_names = ["source", "destination", "owner"])]
    Transfer { amount: u64 },
}
```

### TransactionLogger (recommended)

`TransactionLogger` captures pre/post account state, sends the transaction, decodes, formats, and logs -- all in one call:

```rust
use light_instruction_decoder::{litesvm::TransactionLogger, EnhancedLoggingConfig};

let config = EnhancedLoggingConfig::from_env()
    .with_decoders(vec![Box::new(counter::CounterInstructionDecoder)]);
let logger = TransactionLogger::new(config);

let result = logger.send_transaction(&mut svm, tx);
result.unwrap();
```

`from_env()` enables full debug output when `RUST_BACKTRACE` is set, otherwise uses standard verbosity.

### Callback API

For simpler use cases that don't need pre/post account state diffs:

```rust
use light_instruction_decoder::litesvm::create_logging_callback;

let callback = create_logging_callback(config);

let result = svm.send_transaction(tx.clone());
callback(&tx, &result, &svm);
```

### Manual decode

For full control over state capture and formatting:

```rust
use light_instruction_decoder::litesvm::{
    capture_account_states, decode_transaction, format_transaction,
};

let pre_states = capture_account_states(&svm, &tx);
let result = svm.send_transaction(tx.clone());
let post_states = capture_account_states(&svm, &tx);

let log = decode_transaction(&tx, &result, &config, Some(&pre_states), Some(&post_states));
let formatted = format_transaction(&log, &config, 1);
eprintln!("{formatted}");
```

## Transaction log file

All transactions are logged to `target/instruction_decoder.log` with ANSI escape codes stripped. The file is truncated on the first write per process, then appended for subsequent transactions.

## Console output

Set `RUST_BACKTRACE=1` to print all decoded transactions to stderr during test runs:

```sh
RUST_BACKTRACE=1 cargo test -p my-tests -- --nocapture
```

Failed transactions always print to stderr regardless of `RUST_BACKTRACE`.

## Development

Requires [just](https://github.com/casey/just) and Solana CLI.

```sh
just build       # Build all crates
just test        # Build SBF program + run all tests
just lint        # Check formatting (nightly) + clippy
just format      # Apply nightly formatting
```

## License

Apache-2.0
