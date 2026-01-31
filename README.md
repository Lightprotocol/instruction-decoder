# light-instruction-decoder

Solana instruction decoding library for testing and transaction logging. Decodes raw instruction bytes into human-readable fields, account names, and instruction names.

Supports Anchor programs (8-byte discriminators), native programs (1-byte, 4-byte), and ships with built-in decoders for System Program, SPL Token, Token 2022, Compute Budget, and Light Protocol programs.

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

### Decoding transactions

```rust
use light_instruction_decoder::{DecoderRegistry, EnhancedLoggingConfig};

let config = EnhancedLoggingConfig::default();
let registry = config.decoder_registry();

if let Some((decoded, decoder)) = registry.decode(&program_id, &data, &accounts) {
    println!("{}: {}", decoder.program_name(), decoded.name);
    for field in &decoded.fields {
        println!("  {}: {}", field.name, field.value);
    }
}
```

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
